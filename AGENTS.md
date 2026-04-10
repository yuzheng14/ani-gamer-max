# AGENTS.md

## Cursor Cloud 专用说明

### 项目概览

**Python（`original/`）** 是历史上的原始实现：用于从巴哈姆特动画疯（ani.gamer.com.tw）自动下载动画的 Python 3 应用。该代码已集中在 `original/` 目录，便于对照与迁移。

**Rust 重构** 正在进行：新实现以 Rust（及后续现代前端）为主，与 `original/` 中的 Python 版并行演进直至替代。

**架构设计** 见仓库根目录的 [`architecture.md`](architecture.md)。

Python 原始代码结构（`original/`）：
- 主守护进程（`original/aniGamerPlus.py`）— 自动下载模式 + 命令行模式
- Flask Web 控制台（`original/Dashboard/Server.py`），默认端口 5000
- 核心模块：`original/Config.py`、`original/Anime.py`、`original/Danmu.py`、`original/ColorPrint.py`

### 运行原始 Python 应用

```bash
cd original
# 自动下载模式 + 控制台（默认）
python3 aniGamerPlus.py

# 命令行单集下载
python3 aniGamerPlus.py -s <sn> -m single
```

首次运行前准备：
- 将 `original/config-sample.json` 复制为 `original/config.json`（若缺失可能会自动创建）
- 将 `original/sn_list-sample.txt` 复制为 `original/sn_list.txt`（可选，用于自动模式）
- `ffmpeg` 须在 PATH 中（本环境已预装）

### 重要注意事项

1. **`requirements.txt` 锁定旧版本** — `greenlet==1.1.3` 在 Python 3.12 上无法编译。请不按严格版本锁安装依赖：`pip3 install termcolor flask==1.1.4 pip-system-certs requests chardet flask_basicauth flask_sockets beautifulsoup4 gevent_websocket pysocks lxml 'markupsafe<2.1.0' pyhttpx`。锁定的 `flask==1.1.4` 需要 `markupsafe<2.1.0` 与 `Jinja2==2.11.3`。

2. **网络访问** — 目标站 `ani.gamer.com.tw` 需要台湾地区 IP。若无合适代理，实际视频下载会失败并出现「該 sn 下真的有動畫？」一类错误；控制台及配置与 API 相关功能仍可使用。

3. **控制台监听地址** — 若需外网访问（例如在云主机上测试），在 `config.json` 中将 `dashboard.host` 设为 `"0.0.0.0"`。默认为 `"127.0.0.1"`。

4. **配置自动升级** — `Config.read_settings()` 会将旧版 `config.json` 自动升级。首次使用 `config-sample.json` 运行时，配置版本会从 v13.0 升至 v17.2。

5. **模块级副作用** — `aniGamerPlus.py` 在模块加载时即执行初始化（信号处理、读取配置、读取 sn_list）。被导入时（如 `Dashboard/Server.py`）会触发这些副作用。

6. **无自动化测试** — 本项目没有测试套件。验证方式为运行应用并确认控制台在 `http://localhost:5000/` 可访问。

### 架构参考

- **`architecture.md`** — 当前 Rust 重构的架构设计文档（主参考）。
- **`report/research.md`** — 对原始 Python 实现的逐函数分析与调用链说明。
