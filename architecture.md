# aniGamerPlus Rust 重构：架构与技术栈（讨论稿）

本文档记录当前阶段对 Rust 重构的**模块划分、职责边界、技术选型与一致性约定**，作为后续仓库结构与实现的参考。内容可在实现过程中按实际情况修订。

## 1. 目标与原则

- **单一事实来源**：下载、解析、配置、任务编排等核心业务逻辑只实现在 `agm_core` 中；CLI、HTTP、桌面壳均通过薄封装调用核心，避免在边界层复制业务规则。
- **边界 API 一致**：`agm_desktop`（Tauri 命令/事件）与 `agm_server`（HTTP 接口）对外的**入参、出参、错误语义**尽可能对齐，降低「同一功能两套契约」的维护成本。
- **前端一份代码，两处交付**：`agm-webui` 既作为 Axum 静态资源（浏览器 / NAS / 服务器 Docker），也作为 Tauri 内嵌 WebView 的前端，避免维护两套 UI。

## 2. 组件（crate / 包）划分

| 名称 | 形态 | 职责 |
|------|------|------|
| **agm_core** | Rust library | 核心能力：配置读写与校验、与站点/API 交互、下载与转封装管线、日志与错误类型等。不绑定任何 UI 或具体传输协议。 |
| **agm_cli** | Rust binary | 面向脚本、自动化、无 GUI 环境（含 AI 工具链调用）的 CLI；解析参数后调用 `agm_core`。 |
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

`agm_desktop` 与 `agm_server` 不互相依赖；二者都只依赖 `agm_core`（及各自最小宿主代码）。

## 3. 技术栈建议

### 3.1 Rust 侧

- **Workspace**：单 repo 下用 Cargo workspace 管理 `agm_core`、`agm_cli`、`agm_server`、`agm_desktop` 的宿主 crate（Tauri 侧通常仍有 `src-tauri` crate），便于共享版本与内部 crate 依赖。
- **异步运行时**：`tokio`（与 Axum、多数 HTTP/客户端生态一致）。
- **序列化**：`serde` + `serde_json`；HTTP 与 Tauri 命令边界统一使用 JSON 友好类型，便于前后端与双边界对齐。
- **HTTP**：`axum` + `tower` 生态；静态文件可用 `tower-http::services::ServeDir` 或等价方案。
- **配置**：`agm_core` 内集中定义结构与默认值迁移策略（可与原 Python `Config` 版本升级思路对齐）。
- **错误**：在 `agm_core` 定义统一错误类型（或分层错误），在 CLI 中格式化输出，在 HTTP 中映射为状态码 + 结构化 body，在 Tauri 中映射为可序列化的错误载荷。

### 3.2 前端（agm-webui）

- 具体框架（如 Vue / React / Svelte）待定；选型以 **Tauri 与静态部署兼容性**、团队熟悉度为准。
- **API 客户端**：建议由同一份 OpenAPI/类型定义生成或手写共享 client，对 `agm_server` 的 `fetch` 与 Tauri 侧调用统一抽象（例如统一走 HTTP 到本机 loopback，或 Tauri 下走 `invoke` 但 DTO 与 REST 一致——见下节）。

### 3.3 桌面（agm_desktop）

- **Tauri 2.x**（或团队锁定的主版本）：Rust 侧实现 command handler，内部调用 `agm_core`。
- 权限与打包策略按平台要求配置（自动更新、文件访问等可在后续迭代细化）。

### 3.4 容器与部署（agm_server + agm-webui）

- **镜像**：多阶段构建——一阶段构建 `agm-webui`，一阶段构建 `agm_server` Rust binary，最终镜像包含 binary + 静态资源目录。
- **运行**：单进程即可同时提供 API 与静态 UI；反向代理（可选）仅负责 TLS 与域名，不强制拆服务。

## 4. `agm_server` 与 `agm_desktop` 的契约一致性

### 4.1 为什么要一致

`agm-webui` 在浏览器中主要访问 `agm_server`；在 Tauri 中若希望少写分支，应让**数据形状与错误结构**与 HTTP API 对齐，这样同一套前端逻辑只需切换「传输方式」（HTTP vs `invoke`）。

### 4.2 推荐做法

1. **共享 DTO crate（可选但推荐）**  
   例如 `agm_api_types`（或放在 `agm_core` 的 `api` 模块中），定义请求/响应结构体 + 文档注释；`agm_server` 的 handler 与 `agm_desktop` 的 Tauri command 入参出参**直接使用相同类型**。

2. **REST 资源与命令一一对应**  
   每个 Tauri command 尽量对应一条 REST 路由（同路径语义、同 JSON body），命名上可建立简单对照表（在实现阶段用代码或注释维护）。

3. **错误模型统一**  
   例如统一包含：`code`（机器可读）、`message`（人类可读）、可选 `details`；HTTP 用 4xx/5xx + JSON body；Tauri 返回 `Result<T, ApiError>` 序列化同一结构。

4. **OpenAPI（可选）**  
   从 `agm_server` 生成 OpenAPI 规范，前端与第三方集成共用；Tauri 边界若与 OpenAPI 模型一致，可减少手写重复。

### 4.3 Tauri 下两种集成模式（实现时二选一或并存）

- **模式 A — 本地 HTTP**：桌面壳内仍起小型 loopback HTTP（或复用嵌入式 server），`agm-webui` 始终用 `fetch`，与 NAS 部署完全一致；Tauri 主要负责窗口与系统能力。  
- **模式 B — 直接 invoke**：前端用 `@tauri-apps/api` `invoke`，Rust 侧 command 与 Axum handler **共用同一套 handler 函数**（例如 `agm_core` 提供 `fn handle_x(req) -> Result<Res, E>`，Axum 与 Tauri 只做 JSON 解包/打包）。  

模式 B 更省端口与 CORS 心智负担；模式 A 调试浏览器时与生产 NAS 行为最接近。架构上两种都满足「契约一致」目标。

## 5. agm-webui 的双用途构建

| 场景 | 构建产物 | 使用方式 |
|------|----------|----------|
| 服务器 / Docker | `dist/` 等静态目录 | 由 `agm_server` 挂载为 static，根路径或 `/` 返回 SPA，`/api` 等前缀走 Axum 路由。 |
| Tauri | 同上或 `tauri build` 内嵌 | `dev` 时指向 Vite 等 dev server；`release` 时将同一套 build 打进 `distDir`。 |

注意：

- **环境变量**：API 基地址在「浏览器访问远程 server」与「Tauri 内嵌」下可能不同，通过构建时注入或运行时配置（例如 `agm_server` 同源相对路径 `/api`）统一为一条策略。
- **路由**：若使用前端 history 路由，Axum 侧需对非文件路径回退到 `index.html`（SPA fallback）。

## 6. 与现有 Python 原版的关系

- 业务行为与配置字段以 `original/` 与 `report/research.md` 为对照，在 `agm_core` 中分阶段迁移。
- 本阶段文档**不绑定**具体模块文件名；待 workspace 落地后再在 README 或本文件追加目录树与 crate 依赖图。

## 7. 待决事项（后续讨论）

- 前端框架与组件库的最终选择。
- 是否在首版就引入 OpenAPI 代码生成。
- 认证：仅本机 loopback 是否需要 token；Docker 暴露公网时的鉴权方案。
- 与原版配置文件路径、环境变量名的兼容策略。

---

*文档状态：讨论稿 — 随实现迭代更新。*
