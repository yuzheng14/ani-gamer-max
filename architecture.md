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

### 3.1 Rust 侧

- **Workspace**：单 repo 下用 Cargo workspace 管理 `agm_core`、`agm_cli`、`agm_server`、`agm_desktop` 的宿主 crate（Tauri 侧通常仍有 `src-tauri` crate），便于共享版本与内部 crate 依赖。
- **边界 crate（`agm_cli` / `agm_server` / `agm_desktop`）**：统一视为**薄封装**，不承载业务分支；差异仅在于入口形态（终端参数、HTTP、Tauri command）与对外呈现（文本/JSON/序列化错误）。
- **异步运行时**：`tokio`（与 Axum、多数 HTTP/客户端生态一致）。
- **序列化**：`serde` + `serde_json`；HTTP 与 Tauri 命令边界统一使用 JSON 友好类型，便于前后端与双边界对齐。
- **HTTP**：`axum` + `tower` 生态；静态文件可用 `tower-http::services::ServeDir` 或等价方案。
- **配置**：`agm_core` 内集中定义结构与默认值迁移策略（可与原 Python `Config` 版本升级思路对齐）。
- **错误**：在 `agm_core` 定义统一错误类型（或分层错误），在 CLI 中格式化输出，在 HTTP 中映射为状态码 + 结构化 body，在 Tauri 中映射为可序列化的错误载荷。

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

## 7. 待决事项（后续讨论）

- 与原版配置文件路径、环境变量名的兼容策略。

**认证**：当前阶段**不考虑**鉴权（含 loopback token、Docker 公网暴露等）；若未来暴露不可信网络，再单独设计认证与传输安全。

---

*文档状态：讨论稿（已补充多项已定选型）— 随实现迭代更新。*
