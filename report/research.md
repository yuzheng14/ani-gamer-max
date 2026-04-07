# aniGamerPlus 源碼分析報告

## 項目概述

aniGamerPlus 是一個巴哈姆特動畫瘋（ani.gamer.com.tw）自動下載工具。支持自動監控番劇更新並下載、命令行批量下載、Web Dashboard 管理面板等功能。

入口文件：`aniGamerPlus.py`

---

## 模塊結構

```
aniGamerPlus.py      # 主入口，自動模式 & 命令行模式
├── Config.py        # 配置管理（讀寫配置、cookie、sn_list、版本檢查）
├── Anime.py         # 動畫下載核心（頁面解析、m3u8獲取、下載、上傳、通知）
├── ColorPrint.py    # 彩色輸出 & 日志記錄
├── Danmu.py         # 彈幕下載（.ass 字幕格式）
└── Dashboard/
    └── Server.py    # Flask Web 控制面板（含 WebSocket 任務進度推送）
```

---

## aniGamerPlus.py 全部函數分析

### 1. `port_is_available(port)` — 檢測端口是否可用

**作用**：檢查指定端口是否被佔用（未佔用返回 `True`）。

**調用鏈**：
```
port_is_available(port)
└── socket.connect_ex(('127.0.0.1', port))
```

**被調用位置**：
- `gost_port()` — 為 gost 代理隨機選擇可用端口
- `run_dashboard()` — 啟動 Dashboard 前檢查端口

---

### 2. `gost_port()` — 獲取隨機可用端口

**作用**：在 40000–60000 範圍內隨機選擇一個可用端口，用於 gost 代理的本地監聽。

**調用鏈**：
```
gost_port()
└── port_is_available(random_port)
    └── socket.connect_ex()
```

**被調用位置**：
- 模塊級別全局初始化 `gost_port = gost_port()`
- `build_anime(sn)` 中作為 `gost_port` 參數傳遞給 `Anime()`

---

### 3. `build_anime(sn)` — 構建 Anime 實例

**作用**：根據 sn 碼創建 `Anime` 實例，處理各種異常（抓取失敗、未知錯誤），並執行 sn 解析冷卻。

**調用鏈**：
```
build_anime(sn)
├── Anime(sn, gost_port=gost_port)   # 或 Anime(sn)
│   ├── Config.read_settings()
│   ├── Config.read_cookie()
│   ├── Anime.__init_proxy()          # 如果啟用代理
│   ├── Anime.__init_header()         # 設置 HTTP 請求頭
│   ├── Anime.__get_src()             # 獲取網頁源碼
│   │   └── Anime.__request() / Anime.__request_json()
│   ├── Anime.__get_title()           # 提取頁面標題
│   ├── Anime.__get_bangumi_name()    # 提取番劇名
│   ├── Anime.__get_episode()         # 提取集數
│   └── Anime.__get_episode_list()    # 提取劇集列表
├── anime['anime'].enable_danmu()     # 如果全局 danmu 開啟
├── TryTooManyTimeError 異常處理
│   └── err_print()
├── BaseException 異常處理
│   └── err_print()
└── time.sleep(settings['parse_sn_cd'])  # SN 解析冷卻
```

**被調用位置**：
- `worker()` — 自動模式的下載工作線程
- `check_tasks()` — 檢查番劇更新
- `__download_only()` — 命令行模式純下載
- `__get_info_only()` — 命令行模式查詢信息
- `__cui()` 中的多個下載模式分支

---

### 4. `read_db_all()` — 讀取數據庫所有記錄

**作用**：讀取 SQLite 數據庫中所有動畫記錄，返回包含所有番劇信息的列表。

**調用鏈**：
```
read_db_all()
├── db_locker.acquire()
├── sqlite3.connect(db_path)
├── cursor.execute("select * FROM anime")
├── cursor.fetchall()
└── db_locker.release()
```

**被調用位置**：
- `__cui()` 中 `download_mode == "db"` 分支（用於更新資料庫中所有動畫的彈幕）

---

### 5. `read_db(sn)` — 讀取單條數據庫記錄

**作用**：根據 sn 碼讀取數據庫中對應的動畫記錄。

**調用鏈**：
```
read_db(sn)
├── db_locker.acquire()
├── sqlite3.connect(db_path)
├── cursor.execute("select * FROM anime WHERE sn=:sn", {'sn': sn})
├── cursor.fetchall()[0]
└── db_locker.release()
```

**被調用位置**：
- `worker()` — 檢查下載狀態和上傳狀態
- `check_tasks()` — 檢查番劇是否需要下載

---

### 6. `insert_db(anime)` — 插入數據庫記錄

**作用**：向 SQLite 數據庫插入新的動畫資料記錄。

**調用鏈**：
```
insert_db(anime)
├── db_locker.acquire()
├── anime.get_sn(), anime.get_title(), anime.get_bangumi_name(), anime.get_episode()
├── sqlite3.connect(db_path)
├── cursor.execute("INSERT INTO anime ...")
├── conn.commit()
└── db_locker.release()
```

**被調用位置**：
- `check_tasks()` — 當數據庫中不存在某集記錄時插入

---

### 7. `update_db(anime)` — 更新數據庫記錄

**作用**：更新數據庫中的下載狀態、上傳狀態、分辨率、文件大小、本地路徑等信息。

**調用鏈**：
```
update_db(anime)
├── db_locker.acquire()
├── 判斷 anime.video_size > 5 → status = 1（成功）或 0（失敗）
├── 判斷 anime.upload_succeed_flag → remote_status = 1 或 0
├── sqlite3.connect(db_path)
├── cursor.execute("UPDATE anime SET ...")
├── conn.commit()
└── db_locker.release()
```

**被調用位置**：
- `worker()` — 下載完成後更新、上傳完成後更新、本地文件丟失時更新

---

### 8. `worker(sn, sn_info, realtime_show_file_size=False)` — 自動模式下載工作線程

**作用**：自動模式的核心工作函數，負責從任務隊列中取出任務並執行下載和上傳。

**內部函數**：
- `upload_quit()` — 上傳完成後的清理（從隊列移除、釋放信號量）

**調用鏈**：
```
worker(sn, sn_info, realtime_show_file_size)
├── read_db(sn)                           # 讀取數據庫中該 sn 狀態
│
├── [上傳分支] 如果已下載但未上傳
│   ├── upload_limiter.acquire()
│   ├── build_anime(sn)                   # 構建 Anime 實例
│   ├── os.path.exists(本地文件路徑)       # 檢查文件是否存在
│   ├── anime.upload(bangumi_tag)         # 執行上傳
│   │   └── Anime.upload() → FTP 連接、斷點續傳邏輯
│   ├── update_db(anime)                  # 更新數據庫
│   └── upload_quit()
│
├── [下載分支]
│   ├── thread_limiter.acquire()          # 併發下載限制
│   ├── build_anime(sn)                   # 構建 Anime 實例
│   ├── anime.download(...)               # 執行下載
│   │   ├── Anime.__get_m3u8_dict()       # 獲取 m3u8 清單
│   │   ├── Anime.__segment_download_mode() 或 Anime.__ffmpeg_download_mode()
│   │   ├── 彈幕下載 Danmu.download()
│   │   └── 通知推送（CQ/TG/Discord/Plex）
│   ├── update_db(anime)                  # 更新數據庫
│   ├── download_cd_counter()             # 下載冷卻計時（線程）
│   │
│   ├── [上傳子模塊] 如果配置了上傳
│   │   ├── upload_limiter.acquire()
│   │   ├── anime.upload(bangumi_tag)
│   │   ├── update_db(anime)
│   │   └── upload_limiter.release()
│   │
│   ├── download_cd.join()                # 等待冷卻完成
│   ├── queue.pop(sn)                     # 從任務隊列移除
│   └── processing_queue.remove(sn)       # 從處理中隊列移除
```

**被調用位置**：
- 主循環 `while True` 中（自動模式，每次檢查更新後啟動線程）
- `__cui()` 中的 `list` / `sn-list` 模式

---

### 9. `download_cd_counter()` — 下載冷卻計時器

**作用**：在每次下載完成後進行冷卻等待，避免過於頻繁的下載請求，冷卻完成後釋放下載併發限制器。

**調用鏈**：
```
download_cd_counter()
├── 循環等待 settings['download_cd'] 秒
│   ├── err_print() — 輸出冷卻剩餘時間
│   └── time.sleep(min(30, seconds))
└── thread_limiter.release()  # 釋放併發下載限制
```

**被調用位置**：
- `worker()` — 下載完成後啟動冷卻線程
- `__download_only()` — 命令行模式下載完成後啟動冷卻線程

---

### 10. `check_tasks()` — 檢查番劇更新並生成任務隊列

**作用**：遍歷 `sn_dict`（來自 sn_list.txt），檢查每個番劇是否有需要下載的新集數，將需要下載的 sn 加入全局任務隊列 `queue`。

**調用鏈**：
```
check_tasks()
├── 遍歷 sn_dict.keys()
│   ├── build_anime(sn)                       # 構建 Anime 實例
│   ├── anime.get_bangumi_name()              # 獲取番劇名
│   ├── anime.get_episode_list()              # 獲取劇集列表
│   │
│   ├── [all 模式] 遍歷所有劇集
│   │   ├── read_db(ep)                       # 讀取數據庫狀態
│   │   │   ├── 未下載或未上傳 → queue[ep] = sn_info
│   │   │   └── IndexError → 數據庫無記錄
│   │   │       ├── build_anime(ep)           # 構建新實例
│   │   │       ├── insert_db(new_anime)      # 插入數據庫
│   │   │       └── queue[ep] = sn_info       # 加入隊列
│   │
│   ├── [largest-sn 模式] 選取最大 sn
│   │   └── 同上邏輯，只處理最大 sn 的劇集
│   │
│   ├── [single 模式] 僅處理指定 sn
│   │   └── 同上邏輯
│   │
│   └── [latest 模式] 選取列表最後一集
│       └── 同上邏輯
```

**被調用位置**：
- 主循環 `while True`（自動模式每次更新檢查時調用）
- `__cui()` 中的 `list` / `sn-list` 模式

---

### 11. `__download_only(sn, dl_resolution, dl_save_dir, realtime_show_file_size, classify)` — 純下載函數（命令行模式）

**作用**：僅執行下載操作，不與數據庫交互，支持失敗自動重試（最多3次）。

**調用鏈**：
```
__download_only(sn, ...)
├── thread_limiter.acquire()
├── build_anime(sn)
├── anime.download(resolution, save_dir, ...)
│   ├── Anime.__get_m3u8_dict()
│   ├── Anime.__segment_download_mode() / Anime.__ffmpeg_download_mode()
│   ├── Danmu.download()  （如果啟用彈幕）
│   └── 通知推送
│
├── [失敗重試循環] 最多重試3次
│   ├── err_print() — 輸出失敗信息
│   ├── Config.tasks_progress_rate 更新
│   ├── time.sleep(10)
│   ├── anime.renew()  — 重新獲取頁面信息
│   │   ├── Anime.__get_src()
│   │   ├── Anime.__get_title()
│   │   ├── Anime.__get_bangumi_name()
│   │   ├── Anime.__get_episode()
│   │   └── Anime.__get_episode_list()
│   └── anime.download(...)
│
└── download_cd_counter()  （線程）
```

**被調用位置**：
- `__cui()` — 在 `single`、`latest`、`largest-sn`、`all`、`range`、`sn-range`、`multi` 模式中均有調用

---

### 12. `__get_info_only(sn)` — 僅查詢信息

**作用**：查詢指定 sn 的動畫信息（標題、番劇名、集數、可用分辨率等），不執行下載。

**調用鏈**：
```
__get_info_only(sn)
├── thread_limiter.acquire()
├── build_anime(sn)
├── anime.set_resolution(resolution)
├── anime.get_info()
│   ├── Anime.get_title()
│   ├── Anime.get_bangumi_name()
│   ├── Anime.get_episode()
│   ├── Anime.get_filename()
│   └── Anime.get_m3u8_dict() → Anime.__get_m3u8_dict()
│
├── [如果啟用彈幕]
│   ├── Config.legalize_filename()
│   ├── Danmu(sn, full_filename, Config.read_cookie())
│   └── Danmu.download(settings['danmu_ban_words'])
│
└── thread_limiter.release()
```

**被調用位置**：
- `__cui()` — 當 `--information_only` 參數啟用時

---

### 13. `__get_danmu_only(sn, bangumi_name, video_path)` — 僅下載彈幕

**作用**：為已下載的動畫單獨下載彈幕文件（.ass 字幕格式）。

**調用鏈**：
```
__get_danmu_only(sn, bangumi_name, video_path)
├── thread_limiter.acquire()
├── Config.legalize_filename(bangumi_name)
├── os.path.exists(download_dir) → 檢查番劇資料夾
├── Danmu(sn, video_path.replace(...), Config.read_cookie())
├── Danmu.download(settings['danmu_ban_words'])
│   ├── requests.post('https://ani.gamer.com.tw/ajax/danmuGet.php')
│   ├── requests.get('https://ani.gamer.com.tw/ajax/keywordGet.php')
│   ├── 讀取 DanmuTemplate.ass 模板
│   └── 生成 .ass 彈幕文件
└── thread_limiter.release()
```

**被調用位置**：
- `__cui()` 中 `download_mode == 'danmu'` (即 `db` 模式)

---

### 14. `__cui(...)` — 命令行界面主函數

**作用**：處理命令行模式下的所有下載邏輯，根據不同下載模式分發到對應的處理函數。

**參數**：`sn, cui_resolution, cui_download_mode, cui_thread_limit, ep_range, cui_save_dir, classify, get_info, user_cmd, realtime_show, cui_danmu`

**調用鏈**：
```
__cui(sn, resolution, download_mode, thread_limit, ep_range, ...)
├── 初始化全局 thread_limiter, danmu
│
├── [single 模式]
│   ├── __get_info_only(sn)         # 如果 get_info=True
│   └── __download_only(sn, ...)    # 否則下載
│
├── [latest / largest-sn 模式]
│   ├── build_anime(sn)
│   ├── anime.get_episode_list()
│   ├── bangumi_list.sort()          # largest-sn 排序
│   ├── __get_info_only(最後一集)    # 如果 get_info=True
│   └── __download_only(最後一集, ...)
│
├── [all 模式]
│   ├── build_anime(sn)
│   ├── anime.get_episode_list()
│   └── 遍歷所有劇集 → threading.Thread(__download_only / __get_info_only)
│
├── [range 模式]
│   ├── build_anime(sn)
│   ├── anime.get_episode_list()
│   └── 遍歷指定劇集 → threading.Thread(__download_only / __get_info_only)
│
├── [sn-range 模式]
│   ├── build_anime(sn)
│   ├── anime.get_episode_list() — key/value 互換
│   └── 遍歷指定 sn 範圍 → threading.Thread(__download_only / __get_info_only)
│
├── [multi 模式]
│   └── 遍歷所有指定 sn → threading.Thread(__download_only / __get_info_only)
│
├── [list / sn-list 模式]
│   ├── Config.read_sn_list()
│   ├── check_tasks()              # 生成任務隊列
│   └── 遍歷隊列 → threading.Thread(worker)
│
├── [danmu 模式 (即 db 模式)]
│   └── 遍歷數據庫記錄 → threading.Thread(__get_danmu_only)
│
├── __kill_thread_when_ctrl_c()    # 等待所有線程完成
├── kill_gost()                     # 結束 gost 代理
│
└── [user_cmd] os.popen(settings['user_command'])
```

**被調用位置**：
- `__main__` 中命令行參數解析後
- `Dashboard/Server.py` 中 `manual_task()` 路由（Web 手動任務）

---

### 15. `__kill_thread_when_ctrl_c()` — 等待所有線程完成

**作用**：遍歷 `thread_tasks` 列表，等待所有任務線程完成，使得用戶可以通過 Ctrl+C 中斷程序。

**調用鏈**：
```
__kill_thread_when_ctrl_c()
└── 遍歷 thread_tasks
    └── t.is_alive() → time.sleep(1) 循環等待
```

**被調用位置**：
- `__cui()` — 所有命令行任務完成前

---

### 16. `kill_gost()` — 結束 gost 代理進程

**作用**：如果存在 gost 代理子進程，終止它。

**調用鏈**：
```
kill_gost()
└── gost_subprocess.kill()  # 如果 gost_subprocess 不為 None
```

**被調用位置**：
- `__cui()` — 命令行模式結束前
- `user_exit()` — 用戶 Ctrl+C 退出時

---

### 17. `user_exit(signum, frame)` — 用戶中斷信號處理

**作用**：處理 SIGINT/SIGTERM 信號（用戶按 Ctrl+C），輸出提示信息並結束 gost 代理。

**調用鏈**：
```
user_exit(signum, frame)
├── err_print() — 輸出"你終止了程序!"
├── kill_gost()
└── sys.exit(255)
```

**被調用位置**：
- `signal.signal(signal.SIGINT, user_exit)` — 註冊為 SIGINT 處理函數
- `signal.signal(signal.SIGTERM, user_exit)` — 註冊為 SIGTERM 處理函數

---

### 18. `check_new_version()` — 檢查新版本

**作用**：從 GitHub API 獲取最新版本信息，與當前版本對比，如果有新版則輸出更新信息。

**調用鏈**：
```
check_new_version()
├── Config.read_latest_version_on_github()
│   └── requests.session().get('https://api.github.com/repos/miyouzi/aniGamerPlus/releases/latest')
└── err_print() — 如果有新版，輸出更新內容
```

**被調用位置**：
- `__main__` — 程序啟動時（如果配置 `check_latest_version=True`）

---

### 19. `__init_proxy()` — 初始化代理

**作用**：根據代理配置初始化 gost 代理或直接使用 http/socks5 代理。

**調用鏈**：
```
__init_proxy()
├── [使用 gost 的情況]
│   ├── subprocess.Popen('gost -h')     # 查找 gost
│   ├── 構造 gost 命令
│   ├── threading.Thread(run_gost)       # 後台啟動 gost
│   │   └── subprocess.Popen(gost_cmd)
│   └── time.sleep(3)                    # 等待 gost 啟動
│
└── [不使用 gost 的情況]
    └── print('使用http/https/socks5協議')
```

**被調用位置**：
- `__main__` 中命令行模式的代理初始化
- `__main__` 中自動模式的代理初始化

---

### 20. `do_request(url, headers, cookies, params=None)` — 簡易 HTTP 請求

**作用**：簡單的 requests.get 封裝，用於「我的動畫」導出功能。

**調用鏈**：
```
do_request(url, headers, cookies, params)
└── requests.get(url, headers, cookies, params)
```

**被調用位置**：
- `parse_anime()` — 解析動畫列表頁面
- `export_my_anime()` — 導出我的動畫

---

### 21. `parse_anime(soup, animes, headers, cookies)` — 解析動畫列表

**作用**：從 HTML 解析我的動畫訂閱列表，提取 sn 和名稱。

**調用鏈**：
```
parse_anime(soup, animes, headers, cookies)
├── soup.text.find("目前沒有訂閱內容")   # 檢查是否有訂閱
├── soup.select_one(".theme-list-block").select("a")  # 遍歷動畫條目
│   ├── do_request(animeInfo['href'])    # 訪問每個動畫頁面
│   ├── 提取 sn 從 URL
│   └── animes.append({"sn": sn, "name": name})
└── 返回 True/False
```

**被調用位置**：
- `export_my_anime()` — 導出我的動畫

---

### 22. `export_my_anime()` — 導出「我的動畫」

**作用**：從巴哈姆特獲取用戶的動畫訂閱列表，導出為 `my_anime.txt` 文件。

**調用鏈**：
```
export_my_anime()
├── Config.read_cookie()
├── 分頁循環
│   ├── do_request('https://ani.gamer.com.tw/mygather.php', params={'page': page})
│   ├── BeautifulSoup(html, 'html.parser')
│   └── parse_anime(soup, animes, header, cookies)
└── 寫入 my_anime.txt
```

**被調用位置**：
- `__main__` 中 `--my_anime` 參數

---

### 23. `run_dashboard()` — 啟動 Web 控制面板

**作用**：檢查端口可用性，在後台線程啟動 Flask Web Dashboard。

**調用鏈**：
```
run_dashboard()
├── port_is_available(settings['dashboard']['port'])   # 檢查端口
├── from Dashboard.Server import run as dashboard
├── threading.Thread(target=dashboard).start()
│   └── Dashboard.Server.run()
│       ├── Config.read_settings()
│       ├── Flask 應用配置（BasicAuth、SSL）
│       └── WSGIServer.serve_forever()
├── Config.get_local_ip()
└── err_print() — 輸出訪問地址
```

**被調用位置**：
- `__main__` — 自動模式中，如果 `use_dashboard=True`

---

## 模塊級別初始化（aniGamerPlus.py 頂層代碼）

程序在 import 時即執行以下初始化：

```
signal.signal(signal.SIGINT, user_exit)      # 註冊 Ctrl+C 處理
signal.signal(signal.SIGTERM, user_exit)     # 註冊 SIGTERM 處理
settings = Config.read_settings()             # 讀取配置
working_dir = settings['working_dir']
db_path = os.path.join(working_dir, 'aniGamer.db')
queue = {}                                    # 任務隊列
processing_queue = []                         # 處理中隊列
thread_limiter = threading.Semaphore(...)     # 下載併發限制
upload_limiter = threading.Semaphore(...)     # 上傳併發限制
db_locker = threading.Semaphore(1)            # 數據庫鎖
thread_tasks = []                             # 線程任務列表
gost_subprocess = None                        # gost 子進程
gost_port = gost_port()                       # gost 端口
sn_dict = Config.read_sn_list()              # 讀取 sn_list.txt
danmu = settings['danmu']                     # 彈幕開關
```

---

## `__main__` 主流程

```
if __name__ == '__main__':
│
├── check_new_version()                  # 檢查新版本
├── 初始化 SQLite3 數據庫
│   └── CREATE TABLE IF NOT EXISTS anime (...)
│
├── [命令行模式] len(sys.argv) > 1
│   ├── argparse 解析參數
│   ├── 處理 --my_anime → export_my_anime()
│   ├── 處理保存目錄、分類、清晰度等參數
│   ├── 處理 --episodes 範圍解析
│   ├── Config.test_cookie()             # 測試 cookie
│   ├── __init_proxy()                   # 如果啟用代理
│   └── __cui(...)                        # 進入命令行處理
│
├── [自動模式] 無命令行參數
│   ├── __init_proxy()                   # 如果啟用代理
│   ├── run_dashboard()                  # 啟動 Web 控制面板
│   │
│   └── while True:                       # 主循環
│       ├── Config.test_cookie()         # 測試 cookie
│       ├── [動態讀取] sn_dict = Config.read_sn_list()
│       ├── [動態讀取] settings = Config.read_settings()
│       ├── check_tasks()                # 檢查更新、生成任務隊列
│       ├── 遍歷 queue → threading.Thread(worker).start()
│       ├── err_print() — 輸出任務統計
│       └── time.sleep(check_frequency * 60)  # 冷卻等待
```

---

## 完整調用鏈總覽圖

```
┌─────────────────────────────────────────────────────────────────┐
│                     aniGamerPlus.py (入口)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  模塊級初始化                                                     │
│  ├── signal.signal() → user_exit()                              │
│  ├── Config.read_settings()                                     │
│  ├── gost_port() → port_is_available()                         │
│  ├── Config.read_sn_list()                                     │
│  └── 全局變量初始化 (queue, thread_limiter, etc.)               │
│                                                                  │
│  __main__                                                        │
│  ├── check_new_version() → Config.read_latest_version_on_github()│
│  ├── SQLite3 數據庫初始化                                        │
│  │                                                               │
│  ├── [命令行模式]                                                │
│  │   ├── argparse 參數解析                                       │
│  │   ├── export_my_anime()                                      │
│  │   │   ├── Config.read_cookie()                               │
│  │   │   ├── do_request()                                       │
│  │   │   └── parse_anime()                                      │
│  │   ├── Config.test_cookie()                                   │
│  │   ├── __init_proxy()                                         │
│  │   └── __cui()                                                │
│  │       ├── __download_only() → build_anime() → Anime.download()│
│  │       ├── __get_info_only() → build_anime() → Anime.get_info()│
│  │       ├── __get_danmu_only() → Danmu.download()              │
│  │       ├── check_tasks() → build_anime() → read_db/insert_db  │
│  │       ├── worker() → build_anime() → download/upload         │
│  │       ├── __kill_thread_when_ctrl_c()                        │
│  │       └── kill_gost()                                        │
│  │                                                               │
│  └── [自動模式]                                                  │
│      ├── __init_proxy()                                         │
│      ├── run_dashboard()                                        │
│      │   └── Dashboard.Server.run()                             │
│      │       ├── Flask 路由                                      │
│      │       │   ├── home() → index.html                        │
│      │       │   ├── monitor() → monitor.html                   │
│      │       │   ├── config() → Config.read_settings()          │
│      │       │   ├── recv_config() → Config.write_settings()    │
│      │       │   ├── manual_task() → __cui()                    │
│      │       │   ├── show_sn_list() → Config.get_sn_list_content()│
│      │       │   ├── set_sn_list() → Config.write_sn_list()    │
│      │       │   └── tasks_progress() → WebSocket 推送          │
│      │       └── WSGIServer.serve_forever()                     │
│      │                                                           │
│      └── while True (主循環)                                     │
│          ├── Config.test_cookie()                                │
│          ├── Config.read_sn_list()                               │
│          ├── Config.read_settings()                              │
│          ├── check_tasks()                                       │
│          │   ├── build_anime(sn) → Anime()                      │
│          │   ├── read_db(sn)                                    │
│          │   └── insert_db(anime)                                │
│          ├── worker() (多線程)                                   │
│          │   ├── read_db() / update_db()                        │
│          │   ├── build_anime()                                   │
│          │   ├── Anime.download()                                │
│          │   │   ├── __get_m3u8_dict()                          │
│          │   │   ├── __segment_download_mode()                  │
│          │   │   ├── __ffmpeg_download_mode()                   │
│          │   │   ├── Danmu.download()                           │
│          │   │   └── 通知推送 (CQ/TG/Discord/Plex)              │
│          │   ├── Anime.upload() → FTP                           │
│          │   └── download_cd_counter()                          │
│          └── time.sleep(check_frequency * 60)                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 外部模塊調用關係

### Config.py 被調用的主要函數

| 函數 | 作用 | 調用位置 |
|------|------|---------|
| `read_settings()` | 讀取並驗證配置文件 | 模塊初始化、Anime.__init__、Server.run、多處 |
| `read_cookie()` | 讀取 cookie 文件 | Anime.__init__、Danmu、test_cookie |
| `test_cookie()` | 測試 cookie 是否可讀 | __main__ 主循環、命令行模式 |
| `read_sn_list()` | 讀取番劇訂閱列表 | 模塊初始化、主循環、__cui |
| `legalize_filename()` | 文件名合法化 | Anime 中文件名處理 |
| `read_latest_version_on_github()` | 檢查 GitHub 最新版本 | check_new_version |
| `write_settings()` | 寫入配置文件 | Dashboard recv_config |
| `write_sn_list()` | 寫入 sn_list | Dashboard set_sn_list |
| `get_sn_list_content()` | 讀取 sn_list 原始內容 | Dashboard show_sn_list |
| `renew_cookies()` | 更新 cookie 文件 | Anime.__request cookie 刷新 |
| `invalid_cookie()` | 標記失效 cookie | Anime.__request cookie 失效 |
| `get_working_dir()` | 獲取工作目錄 | Dashboard Server、ColorPrint |
| `get_local_ip()` | 獲取本地 IP | run_dashboard |
| `tasks_progress_rate` | 全局任務進度字典 | Anime.download、worker、Dashboard |

### Anime.py 被調用的主要方法

| 方法 | 作用 | 調用位置 |
|------|------|---------|
| `Anime(sn)` | 構造函數，獲取動畫信息 | build_anime |
| `download()` | 下載動畫視頻 | worker、__download_only |
| `upload()` | FTP 上傳視頻 | worker |
| `get_info()` | 顯示動畫信息 | __get_info_only |
| `get_sn()` | 獲取 sn 碼 | insert_db、update_db 等 |
| `get_title()` | 獲取標題 | insert_db、worker 等 |
| `get_bangumi_name()` | 獲取番劇名 | insert_db、check_tasks |
| `get_episode()` | 獲取集數 | insert_db |
| `get_episode_list()` | 獲取劇集列表 | check_tasks、__cui |
| `enable_danmu()` | 啟用彈幕下載 | build_anime |
| `renew()` | 重新獲取頁面信息 | __download_only 重試 |

### Danmu.py 被調用的主要方法

| 方法 | 作用 | 調用位置 |
|------|------|---------|
| `Danmu(sn, filename, cookies)` | 構造函數 | Anime.download、__get_info_only、__get_danmu_only |
| `download(ban_words)` | 下載彈幕並生成 .ass 文件 | 同上 |

### Dashboard/Server.py 路由

| 路由 | 方法 | 作用 |
|------|------|------|
| `GET /` | `home()` | 主頁面 |
| `GET /monitor` | `monitor()` | 任務監控頁面 |
| `GET /data/config.json` | `config()` | 獲取配置 JSON |
| `POST /uploadConfig` | `recv_config()` | 更新配置 |
| `POST /manualTask` | `manual_task()` | 下達手動任務 → `__cui()` |
| `GET /data/sn_list` | `show_sn_list()` | 獲取 sn_list 內容 |
| `POST /sn_list` | `set_sn_list()` | 更新 sn_list |
| `GET /data/get_token` | `get_token()` | 獲取 WebSocket 鑑權 token |
| `WS /data/tasks_progress` | `tasks_progress()` | WebSocket 推送任務進度 |

---

## 數據流向圖

```
用戶配置文件                    動畫瘋網站
config.json ──→ Config.read_settings()     ani.gamer.com.tw
sn_list.txt ──→ Config.read_sn_list()         │
cookie.txt ──→ Config.read_cookie()            │
                     │                          │
                     ▼                          ▼
              aniGamerPlus.py              Anime.__get_src()
              (主控制器)                   Anime.__get_m3u8_dict()
                     │                          │
                     ▼                          ▼
              check_tasks()              視頻分段下載/ffmpeg下載
              worker()                          │
                     │                          ▼
                     ▼                    本地視頻文件
              aniGamer.db ←── update_db()  bangumi/番劇名/
              (SQLite3)                         │
                                                ▼
                                          FTP 上傳 (可選)
                                          Anime.upload()
                                                │
                                                ▼
                                          通知推送 (可選)
                                          CQ/TG/Discord/Plex
```

---

## 線程模型

```
主線程 (自動模式)
├── Dashboard 線程 (Flask + WebSocket)
├── 更新檢查 → 生成任務隊列
└── 下載工作線程池 (thread_limiter 控制併發)
    ├── worker 線程 1
    │   ├── 下載子線程 (分段下載模式)
    │   └── 上傳子線程
    ├── worker 線程 2
    │   └── ...
    └── worker 線程 N
        └── ...

命令行模式
├── 主線程
└── 下載工作線程池
    ├── __download_only 線程 1
    │   └── 分段下載子線程
    ├── __download_only 線程 2
    └── ...
```
