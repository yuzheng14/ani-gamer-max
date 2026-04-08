# AGENTS.md

## Cursor Cloud specific instructions

### Project Overview

aniGamerPlus is a Python 3 application for automated anime downloading from Bahamut Anime Crazy (ani.gamer.com.tw). The original Python code has been moved to the `original/` directory to prepare for a Rust + modern frontend rewrite.

Original code structure in `original/`:
- Main daemon (`original/aniGamerPlus.py`) — auto-download mode + CLI mode
- Flask Web Dashboard (`original/Dashboard/Server.py`) on port 5000
- Core modules: `original/Config.py`, `original/Anime.py`, `original/Danmu.py`, `original/ColorPrint.py`

### Running the Original Application

```bash
cd original
# Auto-download mode with Dashboard (default)
python3 aniGamerPlus.py

# CLI single-episode download
python3 aniGamerPlus.py -s <sn> -m single
```

Prerequisites before first run:
- Copy `original/config-sample.json` to `original/config.json` (auto-created if missing)
- Copy `original/sn_list-sample.txt` to `original/sn_list.txt` (optional, for auto mode)
- ffmpeg must be in PATH (pre-installed in this environment)

### Important Caveats

1. **requirements.txt pins old versions** — `greenlet==1.1.3` does not compile on Python 3.12. Install deps without strict version pins: `pip3 install termcolor flask==1.1.4 pip-system-certs requests chardet flask_basicauth flask_sockets beautifulsoup4 gevent_websocket pysocks lxml 'markupsafe<2.1.0' pyhttpx`. The pinned `flask==1.1.4` requires `markupsafe<2.1.0` and `Jinja2==2.11.3`.

2. **Network access** — The target site `ani.gamer.com.tw` requires a Taiwan-region IP. Without a suitable proxy, actual video downloads will fail with "該 sn 下真的有動畫？" errors. The Dashboard and all config/API functionality still works.

3. **Dashboard host** — For external access (e.g., testing in a cloud VM), set `dashboard.host` to `"0.0.0.0"` in `config.json`. Default is `"127.0.0.1"`.

4. **Config auto-upgrade** — `Config.read_settings()` automatically upgrades `config.json` from older versions. On first run with `config-sample.json`, the config version will be upgraded from v13.0 to v17.2.

5. **Module-level side effects** — `aniGamerPlus.py` executes initialization at module level (signal handlers, config reading, sn_list reading). Importing it (as `Dashboard/Server.py` does) triggers these side effects.

6. **No automated tests** — This project has no test suite. Verification is done by running the app and checking the Dashboard responds at `http://localhost:5000/`.

### Architecture Reference

See `report/research.md` for a detailed function-by-function analysis and call chain diagram.
