# aniGamerPlus Rust 重构：架构与技术栈（讨论稿）

本文档记录当前阶段对 Rust 重构的**模块划分、职责边界、技术选型与一致性约定**，作为后续仓库结构与实现的参考。内容可在实现过程中按实际情况修订。

## 1. 目标与原则

- **单一事实来源**：下载、解析、配置、任务编排等核心业务逻辑只实现在 `agm_core` 中；CLI、HTTP、桌面壳均通过薄封装调用核心，避免在边界层复制业务规则。
- **边界 API 一致**：`agm_desktop`（Tauri 命令/事件）与 `agm_server`（HTTP 接口）对外的**入参、出参、错误语义**尽可能对齐，降低「同一功能两套契约」的维护成本；`agm_cli` 在对应子命令上复用 **`agm_core` 同一套入口与错误语义**，仅将结果格式化为终端输出与退出码。
- **前端一份代码，两处交付**：`agm-webui` 既作为 Axum 静态资源（浏览器 / NAS / 服务器 Docker），也作为 Tauri 内嵌 WebView 的前端，避免维护两套 UI。

## 2. 组件（crate / 包）划分

| 名称 | 形态 | 职责 |
|------|------|------|
| **agm_core** | Rust library | 核心业务与 **API 契约**（DTO、统一错误）、**各边界共用的 handler**（HTTP / Tauri / CLI 均调用此处）、配置、下载管线等；不绑定 WebView/GUI。`agm_cli` / `agm_server` / `agm_desktop` 只做薄适配。 |
| **agm_cli** | Rust binary | 面向脚本、自动化、无 GUI 环境（含 AI 工具链调用）；**薄适配**：子命令与参数解析（如 `clap`）、终端输出与退出码，业务一律委托 `agm_core`。 |
| **agm_server** | Rust binary | 基于 **Axum** 的 HTTP 服务；路由处理器调用 `agm_core`；对 `agm-webui` 构建产物提供 **static** 托管；可整体容器化部署。 |
| **agm_desktop** | Tauri 应用 | 桌面客户端：系统托盘、本地文件选择、单实例等桌面能力；通过 Tauri 命令（及必要时事件）调用 `agm_core` 或与本地逻辑协作。 |
| **agm-webui** | 前端工程（独立仓库或 workspace 子目录均可） | SPA：任务列表、配置、日志等；**同一套构建**既可复制到 `agm_server` 的静态目录，也可由 Tauri 加载为内嵌前端。 |

依赖方向建议为：

```text
agm-webui（独立构建）
       ↓ 静态资源 / WebView
agm_server ──→ agm_core ←── agm_cli
                  ↑
            agm_desktop（Tauri + 少量宿主逻辑）
```

`agm_cli`、`agm_desktop` 与 `agm_server` **互不依赖**；三者都只依赖 `agm_core`（及各自最小宿主代码）。**共通类型、API 契约与可复用的 handler 逻辑**放在 `agm_core`（必要时再拆 workspace 成员 crate），使 `agm_cli`、`agm_server` 与 `agm_desktop` 尽可能薄。

## 3. 技术栈建议

### 3.1 Rust 侧（已定框架 + 推荐 crate）

- **语言与工程**
  - **Edition**：以 **`Rust 2021`** 为基线（或随首次锁定的 `rust-toolchain` 升到 `2024`，全 workspace 统一）。
  - **Workspace**：单 repo 下 Cargo workspace 管理 `agm_core`、`agm_cli`、`agm_server` 与 Tauri 的 `src-tauri`（`agm_desktop`）；共享 `[workspace.dependencies]` 统一版本。
  - **边界 crate**：`agm_cli` / `agm_server` / `agm_desktop`（Tauri）均为薄封装，见 §2、§4.2。

- **异步与并发**
  - **运行时**：**`tokio`**（`full` 或按需 features），作为 `axum`、站点 HTTP 客户端等与 `agm_core` 异步 API 的底座。
  - **同步热点**：解析、纯计算可留在 async 上下文内或按需 `spawn_blocking`，避免阻塞 worker（实现阶段调优）。

- **HTTP 服务端（`agm_server`）**
  - **框架**：**`axum`**（与 **`tower`**、`tower-http` 组合）。
  - **静态资源**：**`tower-http`** 的 `ServeDir`、以及 SPA **fallback**（非 API 路径回退 `index.html`）。
  - **中间件**：按需 `TraceLayer`、请求体限制、CORS（若未来跨域部署）；当前无认证，不引入 session/JWT 栈。

- **HTTP 客户端（`agm_core` 访问 ani.gamer 等站点）**
  - **与原版对齐**：Python 版在 `original/Anime.py` 中使用 **`pyhttpx.HttpSession(browser_type='firefox'|'chrome')`**，在部分请求路径上走 **浏览器式 TLS / HTTP2 指纹**（JA3 等），以降低被站点或 CDN 侧启发式拦截的概率；普通 **`reqwest`**（尤其默认 **`rustls`**）栈的握手与 ALPN/HTTP2 设置与真实浏览器不一致，**不能等价替代** 上述行为。
  - **推荐（访问目标站点）**：**`rquest`**（由早期的 `reqwest-impersonate` 一脉发展而来，crates.io 上的 **`rquest`**）—— API 与 **`reqwest`** 相近的异步客户端，强调 **TLS / JA3（及 HTTP2 等）指纹模拟**、预置 Chrome / Firefox 等 profile，与「用 pyhttpx 模拟浏览器」的目的一致。**实现阶段以 `rquest` 作为访问 Bahamut 相关域名的默认 HTTP 客户端**，并与配置里的 UA（如沿用「firefox/chrome」分支语义）选择 profile。
  - **备选 / 对照**：生态中还有 **`wreq`** 等侧重指纹与 HTTP/1 细节的客户端，若 `rquest` 在特定环境构建或行为上不满足再评估；**纯 `reqwest`** 仍可用于**不敏感**的出站请求（如健康检查、非风控 URL），以免把 BoringSSL 构建绑到整条链路。
  - **权衡**：`rquest` 类库通常依赖 **BoringSSL**（或等价）路径，**编译时间、跨平台构建与许可证**与「纯 rustls + reqwest」不同；CI 与 Docker 镜像需预留相应依赖或缓存。
  - **代理**：遵循配置与标准代理环境变量（与原版对 `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` 的用法对齐的需求在实现阶段落地）。

- **序列化与 API 边界**
  - **`serde`** + **`serde_json`**：REST 与 Tauri `invoke` 的 JSON 形状与 `agm_core` DTO 一致。
  - **OpenAPI**：**`utoipa`**（及与 Axum 的集成 crate，如 **`utoipa-axum`**）在 `agm_server` 侧**注解路由与类型**，导出 OpenAPI JSON/YAML，供 **`openapi-typescript`（或同类）** 生成前端 TS 类型；`agm_core` 中的类型为单一来源，handler 签名直接复用。

- **配置与持久化（`agm_core`）**
  - **读写 `config.toml` / `sn_list.toml`**：**`serde` + `toml`** 做序列化/反序列化；**程序回写**（服务/桌面经 UI）采用整文件写入时在实现阶段保证原子写（临时文件 + `rename`）。
  - **可选**：**`toml_edit`** 用于 **`agm_cli` 场景**下若需尽量保留注释与键序（与 §6.1「CLI 可手改」一致）；服务/桌面以 UI 为唯一入口时可不依赖。

- **CLI（`agm_cli`）**
  - **参数**：**`clap`**（derive 子命令），输出人类可读错误时消费 `agm_core` 的 `thiserror` 类型。

- **错误处理**
  - **`agm_core`**：**`thiserror`** 导出可序列化、可映射 HTTP 的枚举/结构。
  - **各 binary / Tauri**：边界处可用 **`anyhow`** 收口未预期错误；对外仍转换为统一的 API 错误 DTO。

- **日志与诊断**
  - **`tracing`** + **`tracing-subscriber`**（含 `env-filter`），结构化日志便于 Docker/桌面排错；日志级别与输出格式由 `config.toml` 或环境约定（实现阶段定）。

- **HTML / 文本解析（`agm_core`）**
  - 具体 crate（如 **`scraper`**、**`select`** 等）按 ani.gamer 页面结构在实现时选定；原则是与 async 下载管线分离、可单测。

- **媒体与外部进程**
  - 与原版一致：依赖 **`ffmpeg` 在 PATH**；Rust 侧用 **`tokio::process`**（或 `std::process` 在阻塞任务中）调用，参数构造在 `agm_core` 集中管理。

- **Tauri（`agm_desktop`）**
  - **Tauri 2.x** 当前稳定系列；Rust 侧 **`tauri`**、`tauri-plugin-*` 按需引入；业务仍经 `agm_core`，见 §3.3。

- **测试**
  - **`cargo test`** + 单元测试优先覆盖 `agm_core`；HTTP 层可用 **`tower::ServiceExt::oneshot`** 或 **`axum-test`**（或同类）做集成测；网络相关用录制 fixture / mock server（实现阶段定）。

### 3.2 前端（agm-webui）

- **栈**：**React** + **TanStack Router** + **Vite** + **Tailwind CSS** + **TypeScript**。
- **组件库**：优先 **shadcn/ui**（与 Tailwind 搭配，可复制到项目内的组件源码模式）。
- **API 客户端**：**不采用桌面端 loopback HTTP**。部署在浏览器 / NAS / Docker 时，前端对 `agm_server` 使用常规 **HTTP（`fetch` 等）**；在 **Tauri** 内使用 **`invoke`** 调用 Rust 命令（见 §4.3 模式 B）。两侧共享 **OpenAPI 生成的 TypeScript 类型**（及可选生成的 client），保证请求/响应形状与错误结构与 REST 一致，仅传输层不同。

### 3.3 桌面（agm_desktop）

- **Tauri**：使用当前 **最新的 2.x** 主版本系列；Rust 侧以 **command handler** 为主，内部委托 `agm_core`，与 `agm_cli`、`agm_server` 同为薄边界。
- 权限与打包策略按平台要求配置（自动更新、文件访问等可在后续迭代细化）。

### 3.4 容器与部署（agm_server + agm-webui）

- **镜像**：多阶段构建——一阶段构建 `agm-webui`，一阶段构建 `agm_server` Rust binary，最终镜像包含 binary + 静态资源目录。
- **运行**：单进程即可同时提供 API 与静态 UI；反向代理（可选）仅负责 TLS 与域名，不强制拆服务。

## 4. `agm_server` 与 `agm_desktop` 的契约一致性

### 4.1 为什么要一致

`agm-webui` 在浏览器 / Docker 中通过 **HTTP** 访问 `agm_server`；在 Tauri 中通过 **`invoke`** 调用 Rust。二者应共享**数据形状与错误结构**（由 OpenAPI/生成类型与 `agm_core` 类型保证一致），前端以适配层切换「HTTP `fetch` vs `invoke`」，避免两套业务契约。

### 4.2 已定做法

1. **契约与 DTO 集中在 `agm_core`**  
   请求/响应结构体、统一错误类型，以及 **CLI / HTTP / Tauri 共用的业务 handler**（例如 `fn handle_x(req) -> Result<Res, E>`）均放在 `agm_core`（可按需再拆 workspace 内子 crate，但**不**把「厚逻辑」留在 `agm_cli` / `agm_server` / `agm_desktop`）。各边界 crate 仅负责薄适配：
   - **`agm_cli`**：子命令与参数解析、人类可读输出、进程退出码。
   - **`agm_server`**：路由注册、JSON 解包与打包、HTTP 状态码与静态资源托管。
   - **`agm_desktop`**：Tauri command 注册、JSON 解包与打包、与窗口/系统集成相关能力。

2. **REST 资源与 Tauri command 一一对应**  
   每个 Tauri command 对应一条 REST 路由（同语义、同 JSON body）；可用对照表或命名约定在代码中维护。

3. **错误模型统一**  
   例如统一包含：`code`（机器可读）、`message`（人类可读）、可选 `details`；HTTP 使用 4xx/5xx + JSON body；Tauri 返回 `Result<T, ApiError>` 并序列化为**同一结构**，便于前端共用类型与处理分支。

4. **OpenAPI 与首版代码生成**  
   **首版即引入**：从 `agm_server`（或与 `agm_core` 共享的路由描述）产出 **OpenAPI**，并 **生成 TypeScript 类型**（及按需生成 fetch client），供 `agm-webui` 在 HTTP 场景使用；Tauri `invoke` 的 payload 形状与该规范保持一致，避免两套手写模型。

### 4.3 桌面集成模式（已定）

- **模式 B — `invoke`**：前端使用 `@tauri-apps/api` 的 **`invoke`**；Rust 侧 Tauri command 与 Axum 路由调用 **`agm_core` 中同一套 handler**，不在桌面端另起 HTTP 服务、**不**依赖本机 loopback 访问 API。

浏览器 / 服务器场景仍直接请求 `agm_server` 的 HTTP API；与桌面共享的是 **契约与类型**，不是「桌面也走 HTTP」。

## 5. agm-webui 的双用途构建

| 场景 | 构建产物 | 使用方式 |
|------|----------|----------|
| 服务器 / Docker | `dist/` 等静态目录 | 由 `agm_server` 挂载为 static，根路径或 `/` 返回 SPA，`/api` 等前缀走 Axum 路由。 |
| Tauri | 同上或 `tauri build` 内嵌 | `dev` 时指向 Vite 等 dev server；`release` 时将同一套 build 打进 `distDir`。 |

注意：

- **API 基地址**：仅在 **非 Tauri** 的 HTTP 场景需要配置（如远程 NAS、Docker 暴露的 origin）；Tauri 下走 `invoke`，不配置 loopback URL。HTTP 场景可用构建时注入或同源相对路径（如 `/api`）。
- **路由**：TanStack Router 若使用 history 模式，Axum 托管静态站时需对非文件路径回退到 `index.html`（SPA fallback）。

## 6. 与现有 Python 原版的关系

- 业务行为与配置字段以 `original/` 与 `report/research.md` 为对照，在 `agm_core` 中分阶段迁移。
- 本阶段文档**不绑定**具体模块文件名；待 workspace 落地后再在 README 或本文件追加目录树与 crate 依赖图。

### 6.1 配置文件与清单（已定）

- **主配置**：**`config.toml`**，替代原版 `config.json`。从旧版的字段映射或一次性导入工具可在实现阶段补充。
- **追番清单**：**`sn_list.toml`**，替代原版 `sn_list.txt`；条目结构（如 `[[watch]]` 等）由 `agm_core` 定义。可提供从 `sn_list.txt` 的迁移脚本或首启导入。
- **谁可以手改文件**  
  - **服务模式**（`agm_server`，含 Docker/NAS）与 **客户端模式**（`agm_desktop`）：**不将「人工直接编辑」`config.toml` / `sn_list.toml` 作为支持路径**。配置与清单仅通过 **`agm-webui`**（HTTP 或 Tauri `invoke` 触发的同一套 API）由**程序读写**；磁盘上的 TOML 视为持久化存储，避免与 UI 保存并发手改、格式回写不一致等问题。  
  - **`agm_cli`**：面向脚本与无 GUI 场景，**可**继续支持手改上述 TOML 和/或专用子命令维护（实现阶段细化），与「仅 UI 管理」的服务/桌面模式区分。

### 6.2 环境变量（对照原版）

原版 **没有**为应用设置定义一套「读配置用的」自定义环境变量名；与环境的交互主要是：

- **读取**：`original/Dashboard/Server.py` 会读取 **`ANSI_COLORS_DISABLED`**（常见终端约定，用于关闭彩色输出）；未设置则按默认着色。
- **写入**：`original/Anime.py` 在初始化代理时向进程环境写入标准的 **`HTTP_PROXY`**、**`HTTPS_PROXY`**、**`NO_PROXY`**，以便子进程或依赖这些变量的库走代理；这不是「从环境变量读应用配置」，而是运行时注入。

因此 Rust 版**无需**为「兼容原版自定义 env 键名」单独做对齐；若 HTTP 客户端遵循系统/进程的标准代理变量即可（具体是否在全局设置这些变量，由 `agm_core` 实现决定）。

## 7. 待决事项（后续讨论）

**认证**：当前阶段**不考虑**鉴权（含 loopback token、Docker 公网暴露等）；若未来暴露不可信网络，再单独设计认证与传输安全。

---

*文档状态：讨论稿（已补充多项已定选型）— 随实现迭代更新。*
