# 動畫瘋（ani.gamer.com.tw）接口交互機制完整文檔

> 從原始代碼中逆向整理，作為 Rust 重構的核心依據。

---

## 目錄

- [1. 域名與基礎架構](#1-域名與基礎架構)
- [2. HTTP 請求偽裝](#2-http-請求偽裝)
- [3. Cookie 與身份認證](#3-cookie-與身份認證)
- [4. 影片頁面解析（獲取番劇信息）](#4-影片頁面解析獲取番劇信息)
- [5. 視頻流獲取完整流程（核心）](#5-視頻流獲取完整流程核心)
- [6. 視頻下載與解密](#6-視頻下載與解密)
- [7. 彈幕系統](#7-彈幕系統)
- [8. 我的動畫導出](#8-我的動畫導出)
- [9. 反爬與地區限制](#9-反爬與地區限制)
- [10. 代理架構](#10-代理架構)

---

## 1. 域名與基礎架構

| 域名 | 用途 |
|------|------|
| `ani.gamer.com.tw` | 主站，Web 頁面和所有 AJAX 接口 |
| `api.gamer.com.tw` | 移動端 APP API |
| `bahamut.akamaized.net` | Akamai CDN，視頻流分發（m3u8 + ts 分段） |
| `home.gamer.com.tw` | 巴哈姆特主站，Cookie 登錄設備管理 |

視頻流走 Akamai CDN，配置項 `no_proxy_akamai` 控制是否對 CDN 也走代理。如果設為 `true`，則 `NO_PROXY` 環境變量會包含 `bahamut.akamaized.net`。

---

## 2. HTTP 請求偽裝

系統支持兩套 Header 模式，通過 `use_mobile_api` 配置切換：

### Web Header（默認）

```
User-Agent: {用戶配置的 UA，需與獲取 cookie 的瀏覽器一致}
Referer: https://ani.gamer.com.tw/animeVideo.php?sn={sn}
Accept-Language: zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.6
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8
Accept-Encoding: gzip, deflate
Cache-Control: max-age=0
Origin: https://ani.gamer.com.tw
```

### Mobile Header（APP API）

```
User-Agent: Animad/1.16.16 (tw.com.gamer.android.animad; build:328; Android 9) okHttp/4.4.0
X-Bahamut-App-Android: tw.com.gamer.android.animad
X-Bahamut-App-Version: 328
Accept-Encoding: gzip
Connection: Keep-Alive
```

### TLS 指紋偽裝

Web 模式下的首次頁面請求使用 `pyhttpx`（而非普通 `requests`）發起，這是一個能模擬瀏覽器 TLS 指紋（JA3）的 HTTP 客戶端。根據 UA 中是否包含 "firefox" 來選擇模擬的瀏覽器類型：

```python
if 'firefox' in ua.lower():
    pyhttpx.HttpSession(browser_type='firefox')
else:
    pyhttpx.HttpSession(browser_type='chrome')
```

**重構要點**：Rust 端需要使用支持 TLS 指紋偽裝的 HTTP 客戶端（如 `reqwest` 配合自定義 TLS 配置，或使用 `boring` crate），否則可能被動畫瘋的反爬識別。

---

## 3. Cookie 與身份認證

### 3.1 Cookie 來源

用戶需從瀏覽器無痕模式登錄動畫瘋後，手動提取 Cookie 字符串保存為 `cookie.txt`。

Cookie 格式（一行，分號分隔）：
```
key1=value1; key2=value2; key3=value3
```

解析邏輯：
1. 按 `"; "` 分割
2. 每項按 `"="` 分割（只分割第一個 `=`，值中可能含 `=`）
3. 如果值中包含中文字符，需進行 URL 編碼（`percent-encode`）
4. 刪除 `ckBH_lastBoard` 這個不需要的 Cookie

### 3.2 關鍵 Cookie 字段

| Cookie | 含義 |
|--------|------|
| `BAHAID` | 已登錄用戶的帳號標識 |
| `BAHARUNE` | 登錄會話令牌（核心，用於判斷 Cookie 是否刷新） |
| `nologinuser` | 遊客身份 Cookie（未登錄時由服務器設置） |

### 3.3 Cookie 自動刷新機制

**這是最複雜的交互邏輯之一，直接影響長期運行的穩定性。**

觸發條件：服務器響應中包含 `set-cookie` 頭。

**場景一：本線程收到新 Cookie**
```
if 'set-cookie' in response.headers:
    if 'deleted' NOT in set-cookie:
        # 本線程率先收到新 Cookie
        1. 用 session.cookies 更新本地 cookies
        2. 調用 Config.renew_cookies() 寫入 cookie.txt
        3. 訪問一次 https://ani.gamer.com.tw/ 完成刷新
        4. 檢查 set-cookie 中是否包含 'BAHARUNE' 確認刷新成功
```

**場景二：其他線程先收到了 Cookie（本線程收到 `deleted`）**
```
if 'deleted' in set-cookie:
    # 刷新機會已被其他線程使用
    1. 等待 2 秒（讓其他線程完成寫入）
    2. 最多重試 3 次從 cookie.txt 重新讀取
    3. 對比 BAHARUNE 值是否變化
    4. 如果 3 次都失敗 → 標記 Cookie 失效，回退為遊客模式
       - Cookie 文件重命名為 invalid_cookie.txt
```

**場景三：移動端 API 模式下 Cookie 刷新**
```
移動端 Header 無法刷新 Cookie
→ 臨時切換為 Web Header
→ 請求 https://ani.gamer.com.tw/ 嘗試刷新
→ 刷新完成後切回 Mobile Header
→ 如果切換 Header 也失敗，恢復 Mobile Header（廣告只需 3s）
```

**重構要點**：Cookie 刷新是多線程並發安全問題，需要原子操作或鎖機制。Rust 端建議使用 `Arc<RwLock<CookieJar>>` 共享 Cookie 狀態。

---

## 4. 影片頁面解析（獲取番劇信息）

### 4.1 Web 模式

**請求**：
```
GET https://ani.gamer.com.tw/animeVideo.php?sn={sn}
Cookie: 不攜帶（no_cookies=True）
使用 pyhttpx（TLS 指紋偽裝）
```

**HTML 解析**：
- 標題：`soup.find('div', 'anime_name').h1.string` → 格式如 `"番劇名 [集數]"`
- 當前集數：`soup.find('li', 'playing').a.string`（劇集列表存在時）
- 劇集列表：`soup.find('section', 'season').find_all('a')`
  - 每個 `<a>` 的 `href` 格式為 `?sn=12345`，提取 sn
  - `<a>` 的文本為集數名（字符串，可能是數字、"特別篇"、"電影" 等）
  - 如果有 `<p>` 標籤，表示劇集類型分組（"本篇"、"特別篇" 等）
- 只有一集時無 `<section class="season">`，劇集列表僅包含自身

### 4.2 Mobile API 模式

**請求**：
```
GET https://api.gamer.com.tw/mobile_app/anime/v4/video.php?sn={sn}
Cookie: 不攜帶
```

**JSON 響應結構**：
```json
{
  "data": {
    "anime": {
      "title": "番劇名 [集數]",
      "episodes": {
        "0": [{"episode": "1", "videoSn": 12345}],   // 本篇
        "1": [{"episode": "", "videoSn": 12346}],     // 電影
        "2": [{"episode": "1", "videoSn": 12347}],    // 特別篇
        "3": [{"episode": "1", "videoSn": 12348}],    // 中文配音
        "4": [{"episode": "", "videoSn": 12349}]      // 中文電影
      }
    }
  }
}
```

**劇集類型對應**：

| type 值 | 含義 | key 格式 |
|---------|------|---------|
| `"0"` | 本篇 | `"{episode}"` |
| `"1"` | 電影 | `"電影"` |
| `"2"` | 特別篇 | `"特別篇{episode}"` |
| `"3"` | 中文配音 | `"中文配音{episode}"` |
| 其他 | 中文電影 | `"中文電影"` |

### 4.3 標題解析規則

從標題 `"番劇名 [集數]"` 中提取集數：

1. 優先匹配 `\[\d*\.?\d* *\.?[A-Z,a-z]*(?:電影)?\]`（數字集數，含小數、字母後綴、"電影"）
2. 次選 `\[.+?\]`（任意非空中括號內容）
3. 都不匹配則集數默認為 `"1"`

番劇名 = 標題去掉 `[集數]` 後的部分，去除首尾空格和重復空格。

---

## 5. 視頻流獲取完整流程（核心）

這是整個程序最關鍵的交互鏈，需要嚴格按順序執行。

### 5.1 時序圖

```
客戶端                              ani.gamer.com.tw / Akamai CDN
  │
  │──── Step 1: GET /ajax/getdeviceid.php ──────────────→│
  │←──── {"deviceid": "xxx"} ───────────────────────────│
  │
  │──── Step 2: GET /ajax/token.php?adID=0&sn=X         │
  │              &device=xxx&hash=yyy ──────────────────→│
  │←──── {"vip": true/false, ...} ──────────────────────│
  │                                                       │
  │     [僅 Web 模式]                                     │
  │──── Step 3a: GET /ajax/unlock.php?sn=X&ttl=0 ──────→│  (×3次)
  │──── Step 3b: GET /ajax/checklock.php?device=X&sn=Y ─→│
  │──── Step 3c: GET /ajax/unlock.php?sn=X&ttl=0 ──────→│  (×2次)
  │                                                       │
  │     [如果非 VIP]                                      │
  │──── Step 4a: 開始廣告                                 │
  │     Web:  GET /ajax/videoCastcishu.php?sn=X&s=194699 │
  │     APP:  GET /mobile_app/anime/v1/stat_ad.php       │
  │                ?schedule=-1&sn=X ──────────────────→│
  │                                                       │
  │     time.sleep(ads_time)  // Web:25s, APP:25s        │
  │                                                       │
  │──── Step 4b: 跳過廣告                                 │
  │     Web:  GET /ajax/videoCastcishu.php               │
  │                ?sn=X&s=194699&ad=end ──────────────→│
  │     APP:  GET /mobile_app/anime/v1/stat_ad.php       │
  │                ?schedule=-1&ad=end&sn=X ────────────→│
  │                                                       │
  │     [僅 Web 模式]                                     │
  │──── Step 5: GET /ajax/videoStart.php?sn=X ──────────→│
  │                                                       │
  │     [僅 Web 模式 - 廣告驗證循環]                       │
  │──── Step 6: GET /ajax/token.php                       │
  │              ?sn=X&device=xxx&hash=zzz ─────────────→│
  │←──── {"time": 1} ──────────────────────────────────│  (time==1 表示通過)
  │     如果 time != 1: 等 2s → 重試 skip_ad + videoStart │
  │     最多重試 10 次                                     │
  │     如果沒有 'time' 字段 → 地區限制                    │
  │                                                       │
  │──── Step 7: 獲取播放列表                               │
  │     Web:  GET /ajax/m3u8.php?sn=X&device=xxx ──────→│
  │     APP:  GET /mobile_app/anime/v3/m3u8.php          │
  │                ?videoSn=X&device=xxx ───────────────→│
  │←──── {"src": "https://...playlist.m3u8"} ───────────│
  │      APP: {"data": {"src": "https://...playlist.m3u8"}}│
  │                                                       │
  │──── Step 8: GET playlist.m3u8 ──────────────────────→│ (Akamai CDN)
  │←──── 多分辨率 m3u8 索引 ────────────────────────────│
  │                                                       │
  │──── Step 9: GET chunklist_xxx.m3u8 ─────────────────→│ (選定分辨率)
  │←──── 分段列表 + AES-128 key URI ───────────────────│
  │                                                       │
  │──── Step 10: GET key URI ───────────────────────────→│ (下載解密 key)
  │──── Step 11: GET media_b*.ts (×N 個分段) ───────────→│ (下載視頻分段)
  │                                                       │
  │     Step 12: ffmpeg 本地解密合併                        │
```

### 5.2 各步驟詳細接口

#### Step 1: 獲取設備 ID

```
GET https://ani.gamer.com.tw/ajax/getdeviceid.php
Cookie: 攜帶用戶 Cookie
```

**響應**：
```json
{"deviceid": "一串設備ID字符串"}
```

#### Step 2: 獲取訪問令牌（gain_access）

Web 模式：
```
GET https://ani.gamer.com.tw/ajax/token.php?adID=0&sn={sn}&device={device_id}&hash={random_12位字符串}
```

APP 模式：
```
GET https://ani.gamer.com.tw/ajax/token.php?adID=0&sn={sn}&device={device_id}
```

`hash` 生成方式：12 位隨機字符串，字符集為 `abcdefghijklmnopqrstuvwxyz0123456789`，種子為 `int(time.time() * 1000)`。

**響應**：
```json
{
  "vip": true,          // 是否為 VIP 帳號
  "time": 1,            // 1 表示可以開始下載
  "error": {            // 出錯時存在
    "code": 1,
    "message": "..."
  }
}
```

**錯誤場景**：如果響應中沒有 `time` 字段 → IP 被地區限制。

#### Step 3: 解鎖（僅 Web 模式）

按照固定順序執行：
```
unlock → checklock → unlock → unlock
```

```
GET https://ani.gamer.com.tw/ajax/unlock.php?sn={sn}&ttl=0
GET https://ani.gamer.com.tw/ajax/checklock.php?device={device_id}&sn={sn}
```

無有意義的響應正文。

#### Step 4: 廣告處理（非 VIP）

**開始廣告**：

Web 模式：
```
GET https://ani.gamer.com.tw/ajax/videoCastcishu.php?sn={sn}&s=194699
```

APP 模式：
```
GET https://api.gamer.com.tw/mobile_app/anime/v1/stat_ad.php?schedule=-1&sn={sn}
```

**等待廣告**：`time.sleep(ads_time)`
- Web 默認 25 秒（歷史：8s → 20s → 25s）
- APP 默認 25 秒（最低可到 3s）

**跳過廣告**：

Web 模式：
```
GET https://ani.gamer.com.tw/ajax/videoCastcishu.php?sn={sn}&s=194699&ad=end
```

APP 模式：
```
GET https://api.gamer.com.tw/mobile_app/anime/v1/stat_ad.php?schedule=-1&ad=end&sn={sn}
```

#### Step 5: 視頻開始（僅 Web 模式）

```
GET https://ani.gamer.com.tw/ajax/videoStart.php?sn={sn}
```

#### Step 6: 廣告驗證（僅 Web 模式，check_no_ad）

```
GET https://ani.gamer.com.tw/ajax/token.php?sn={sn}&device={device_id}&hash={random_12位}
```

**響應判斷**：
- `response['time'] == 1` → 廣告已去除，可以繼續
- `response['time'] != 1` → 追加等待 2 秒，重新 `skip_ad()` + `video_start()`，最多重試 10 次
- 響應中無 `time` 字段 → IP 被地區限制

如果重試後成功，程序會自適應更新 `ads_time` 配置並保存：
```
新 ads_time = (10 - 剩餘重試次數) × 2 + 原 ads_time + 2
```

#### Step 7: 獲取播放列表 URL

Web 模式：
```
GET https://ani.gamer.com.tw/ajax/m3u8.php?sn={sn}&device={device_id}
```

APP 模式：
```
GET https://api.gamer.com.tw/mobile_app/anime/v3/m3u8.php?videoSn={sn}&device={device_id}
```

**響應**：
```json
// Web:
{"src": "https://bahamut.akamaized.net/.../playlist.m3u8?..."}

// APP:
{"data": {"src": "https://bahamut.akamaized.net/.../playlist.m3u8?..."}}
```

#### Step 8: 解析主播放列表（多分辨率索引）

```
GET {playlist_url}
Cookie: 不攜帶
Header 額外添加: Origin: https://ani.gamer.com.tw
```

**響應**（標準 HLS m3u8 格式）：
```
#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=...,RESOLUTION=640x360
chunklist_360p.m3u8?...
#EXT-X-STREAM-INF:BANDWIDTH=...,RESOLUTION=1280x720
chunklist_720p.m3u8?...
#EXT-X-STREAM-INF:BANDWIDTH=...,RESOLUTION=1920x1080
chunklist_1080p.m3u8?...
```

**解析邏輯**：
1. 正則 `=\d+x\d+\n.+` 提取分辨率行和對應的 chunklist 文件名
2. 從 `=WIDTHxHEIGHT` 中提取縱向像素數 `HEIGHT` 作為 key（如 `"360"`, `"720"`, `"1080"`）
3. 拼接完整 URL：`url_prefix + chunklist文件名`
   - `url_prefix` = playlist URL 去掉 `playlist` 及之後的部分

**分辨率選擇**：
- 如果指定分辨率存在 → 直接使用
- 如果不存在且 `lock_resolution=false` → 選取**最接近**的分辨率（非最高）
- 如果不存在且 `lock_resolution=true` → 取消下載

---

## 6. 視頻下載與解密

### 6.1 分段下載模式（推薦，`segment_download_mode=true`）

**Step 9: 請求分段 m3u8**：
```
GET {chunklist_url}
Cookie: 不攜帶
```

**解析**：
1. 提取 AES-128 key URI：正則 `(?<=AES-128,URI=")(.*)(?=")`
2. 如果 key URI 不是完整 URL（不以 `http` 開頭），拼接 `url_prefix + '/' + key_uri`
3. 提取所有分段文件名：正則 `media_b.+ts.*`

**Step 10: 下載 key**：
```
GET {key_uri}
Cookie: 不攜帶
→ 保存為二進制文件 key.m3u8key
```

**Step 11: 併發下載分段**：
```
GET {url_prefix}/{chunk_filename}
Cookie: 不攜帶
最大重試次數: segment_max_retry (默認 8, -1 為無限)
併發數: multi_downloading_segment (默認 2, 最高 5)
```

**Step 12: ffmpeg 解密合併**：
```bash
ffmpeg -allowed_extensions ALL -i {本地化m3u8} -c copy {output} -y

# 可選參數:
-movflags faststart          # metadata 前置（僅 mp4）
-metadata:s:a:0 language=jpn  # 日語音軌標籤（標題不含"中文"時）
-metadata:s:a:0 language=chi  # 中文音軌標籤（標題含"中文"時）
```

m3u8 本地化：將 m3u8 文件中的遠程 key URI 和 chunk 文件名替換為本地路徑。

### 6.2 ffmpeg 直接下載模式（`segment_download_mode=false`）

```bash
ffmpeg -user_agent "{ua}" \
       -headers "Origin: https://ani.gamer.com.tw" \
       -i {chunklist_url} \
       -c copy {output} -y
```

卡死判定：1 分鐘內文件大小增長不超過 3MB → 判定為卡死，kill ffmpeg。

---

## 7. 彈幕系統

### 7.1 獲取彈幕數據

```
POST https://ani.gamer.com.tw/ajax/danmuGet.php
Content-Type: application/x-www-form-urlencoded;charset=utf-8
Origin: https://ani.gamer.com.tw
Authority: ani.gamer.com.tw
User-Agent: Chrome/85...

Body: sn={sn}
```

**響應**（JSON 數組）：
```json
[
  {
    "text": "彈幕文字內容",
    "time": 125,           // 時間戳，單位：0.1秒（即 12.5 秒）
    "color": "#FFFFFF",    // RGB 顏色碼
    "position": 0          // 0=滾動, 1=頂部, 2=底部
  },
  ...
]
```

### 7.2 獲取線上過濾關鍵詞

```
GET https://ani.gamer.com.tw/ajax/keywordGet.php
Accept: application/json
Origin: https://ani.gamer.com.tw
Cookie: 攜帶用戶 Cookie
```

**響應**：
```json
[
  {"keyword": "過濾詞1"},
  {"keyword": "過濾詞2"}
]
```

### 7.3 彈幕時間換算

```
原始 time 值 → 除以 10 得到秒數，模 10 得到百毫秒
示例：time=125 → 12秒.5（即 0:00:12.50）
```

### 7.4 彈幕位置類型

| position | 含義 | ASS Style | 行為 |
|----------|------|-----------|------|
| 0 | 滾動彈幕 | `Roll` | 從右到左移動，`\move(1920,Y,-1000,Y)`，持續 10-14 秒隨機 |
| 1 | 頂部彈幕 | `Top` | 固定頂部，持續 5 秒 |
| 2 | 底部彈幕 | `Bottom` | 固定底部，持續 5 秒 |

### 7.5 彈幕顏色處理

原始為 `#RRGGBB` 格式，ASS 字幕需要 `BGR` 順序：
```
RGB "#FF8800" → BGR "0088FF"
ASS 格式: \1c&H4C{BGR}
```

---

## 8. 我的動畫導出

```
GET https://ani.gamer.com.tw/mygather.php?page={page}&sort=0
Cookie: 攜帶用戶 Cookie
Accept: application/json
```

**HTML 解析**：
- 無訂閱：頁面包含文字 `"目前沒有訂閱內容"`
- 有訂閱：`.theme-list-block` 下的所有 `<a>` 標籤
  - 每個 `<a>` 的 `href` 指向動畫頁面
  - 需要再次請求該 URL，從最終重定向後的 URL 中提取 sn：`response.url.split("=")[-1]`
  - `.theme-name` 提取番劇名
- 分頁：page 遞增直到無更多內容

---

## 9. 反爬與地區限制

### 9.1 地區限制

- 動畫瘋僅限台灣地區 IP 訪問
- 判斷方式：`token.php` 響應中無 `time` 字段 → 地區限制
- 錯誤信息：`"遭到動畫瘋地區限制, 你的IP可能不被動畫瘋認可!"`

### 9.2 限制級動畫

- 需要登錄才能觀看
- 判斷方式：`token.php` 響應中存在 `error` 字段
  - `error.code` 和 `error.message` 描述具體錯誤

### 9.3 TLS 指紋檢測

- 主頁面請求使用 `pyhttpx` 模擬瀏覽器 TLS 指紋
- 其他 AJAX 請求使用普通 `requests`
- CDN 請求（Akamai）不攜帶 Cookie

### 9.4 請求節奏控制

- `parse_sn_cd`：每次解析 sn 頁面後的冷卻時間（默認 3-5 秒）
- `download_cd`：每次下載完成後的冷卻時間（默認 60 秒）
- `ads_time` / `mobile_ads_time`：廣告等待時間（默認 25 秒）

---

## 10. 代理架構

### 10.1 原生支持的代理協議

- `http://`
- `https://`
- `socks5://`
- `socks5h://`（遠程 DNS 解析）

格式：
```
http://user:passwd@example.com:1000    # 帶認證
socks5h://127.0.0.1:1483              # SOCKS5 遠程 DNS
```

### 10.2 gost 擴展代理

如果代理協議不是上述四種（如 `ss://`、`vmess://` 等），則啟用 gost：

```bash
gost -L=:{隨機端口40000-60000} -F={用戶配置的代理}
```

程序自身連接 gost 本地端口（`http://127.0.0.1:{port}`），gost 負責轉發到上游。

### 10.3 Akamai CDN 代理控制

配置項 `no_proxy_akamai`：
- `false`（默認）：CDN 流量也走代理
- `true`：CDN 流量不走代理（`NO_PROXY` 包含 `bahamut.akamaized.net`）

---

## 附：接口清單速查表

| # | 方法 | URL | 用途 | Cookie | 模式 |
|---|------|-----|------|--------|------|
| 1 | GET | `ani.gamer.com.tw/animeVideo.php?sn={sn}` | 獲取番劇頁面 | ✗ (pyhttpx) | Web |
| 2 | GET | `api.gamer.com.tw/mobile_app/anime/v4/video.php?sn={sn}` | 獲取番劇信息 | ✗ | APP |
| 3 | GET | `ani.gamer.com.tw/ajax/getdeviceid.php` | 獲取設備 ID | ✓ | 兩者 |
| 4 | GET | `ani.gamer.com.tw/ajax/token.php?adID=0&sn={sn}&device={did}&hash={h}` | 獲取訪問令牌 | ✓ | 兩者 |
| 5 | GET | `ani.gamer.com.tw/ajax/unlock.php?sn={sn}&ttl=0` | 解鎖 | ✓ | Web |
| 6 | GET | `ani.gamer.com.tw/ajax/checklock.php?device={did}&sn={sn}` | 檢查鎖定 | ✓ | Web |
| 7 | GET | `ani.gamer.com.tw/ajax/videoCastcishu.php?sn={sn}&s=194699` | 開始廣告 | ✓ | Web |
| 8 | GET | `ani.gamer.com.tw/ajax/videoCastcishu.php?sn={sn}&s=194699&ad=end` | 跳過廣告 | ✓ | Web |
| 9 | GET | `api.gamer.com.tw/mobile_app/anime/v1/stat_ad.php?schedule=-1&sn={sn}` | 開始廣告 | ✓ | APP |
| 10 | GET | `api.gamer.com.tw/mobile_app/anime/v1/stat_ad.php?schedule=-1&ad=end&sn={sn}` | 跳過廣告 | ✓ | APP |
| 11 | GET | `ani.gamer.com.tw/ajax/videoStart.php?sn={sn}` | 視頻開始 | ✓ | Web |
| 12 | GET | `ani.gamer.com.tw/ajax/m3u8.php?sn={sn}&device={did}` | 獲取播放列表 | ✓ | Web |
| 13 | GET | `api.gamer.com.tw/mobile_app/anime/v3/m3u8.php?videoSn={sn}&device={did}` | 獲取播放列表 | ✓ | APP |
| 14 | GET | `{playlist_url}` (Akamai CDN) | 主播放列表 | ✗ | 兩者 |
| 15 | GET | `{chunklist_url}` (Akamai CDN) | 分段列表 | ✗ | 兩者 |
| 16 | GET | `{key_uri}` (Akamai CDN) | AES-128 解密 key | ✗ | 兩者 |
| 17 | GET | `{chunk_uri}` (Akamai CDN) | 視頻 ts 分段 | ✗ | 兩者 |
| 18 | POST | `ani.gamer.com.tw/ajax/danmuGet.php` | 獲取彈幕 | ✗ | 兩者 |
| 19 | GET | `ani.gamer.com.tw/ajax/keywordGet.php` | 獲取彈幕過濾詞 | ✓ | 兩者 |
| 20 | GET | `ani.gamer.com.tw/mygather.php?page={p}&sort=0` | 我的動畫 | ✓ | 兩者 |
| 21 | GET | `ani.gamer.com.tw/` | Cookie 刷新觸發 | ✓ | 兩者 |
