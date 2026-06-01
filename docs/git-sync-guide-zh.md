# Git 资产同步使用指南

## 功能介绍

Git 同步把你的 **AI agent 资产**以**明文目录**形式提交到一个**私有 Git 仓库**，让你「配置一次、换机器无缝恢复」。它取代了旧版的 WebDAV 同步。

同步内容：

| 资产 | 形态 | 说明 |
| --- | --- | --- |
| 供应商配置（Claude / Codex / Gemini / OpenCode / OpenClaw / Hermes） | `db.sql` + `providers/<app>.json` | `db.sql` 为无损权威备份；`providers/*.json` 为可读镜像 |
| MCP 服务器 | `db.sql` + `mcp/servers.json` | 同上 |
| Skills（技能） | `skills/<name>/...` | SSOT 技能内容的明文目录 |
| Claude `settings.json`（含 hooks）、`agents/`（子代理）、`commands/`（斜杠命令）、`CLAUDE.md`、`output-styles/` | `agents/claude/...` | 仅跨机镜像，不做跨工具转换 |
| Codex `AGENTS.md`、`prompts/` | `agents/codex/...` | 同上 |
| Gemini `GEMINI.md`、`commands/` | `agents/gemini/...` | 同上 |

> **设计原则**：供应商 / MCP / Skills 始终同步；其余「agent 资产」按工具原样备份/恢复，可在设置里逐项勾选开关，**不做跨工具格式转换**。

## 前置要求

- 本机已安装 **git**（命令行可用）。
- 一个**私有** Git 仓库（GitHub / GitLab / 自建均可），以及一个 **Personal Access Token (PAT)**，需要 `repo`（读写）权限。

> ⚠️ **务必使用私有仓库**：PAT 仅保存在本机，但**供应商配置中的 API Key 会以明文提交**到仓库。

## 如何获取 PAT

PAT（Personal Access Token）用于授权本应用读写你的同步仓库。按托管平台选择对应步骤，最少只需要**仓库内容读写**权限。

> 💡 token **只在创建时显示一次**，请立即复制保存；它仅存于本机，不会进入同步仓库。

### GitHub

**方式 A：Fine-grained token（推荐，权限更精细）**

1. 右上角头像 → **Settings**
2. 左侧最下 **Developer settings**
3. **Personal access tokens → Fine-grained tokens → Generate new token**
4. 填写：
   - **Token name**：如 `olenro-sync`
   - **Expiration**：按需（如 90 天或自定义）
   - **Repository access** → **Only select repositories** → 选你的**私有同步仓库**
   - **Permissions → Repository permissions → Contents** 设为 **Read and write**（git 推/拉必需；其余保持 No access）
5. **Generate token**，复制 `github_pat_...`

**方式 B：Classic token（更简单）**

1. **Settings → Developer settings → Personal access tokens → Tokens (classic) → Generate new token (classic)**
2. 勾选 **`repo`** 大类（含私有仓库读写）
3. 生成并复制 `ghp_...`

直达链接：`https://github.com/settings/tokens`

### GitLab

1. 头像 → **Edit profile → Access Tokens**（或 `https://gitlab.com/-/user_settings/personal_access_tokens`）
2. 勾选 scope：**`read_repository`** + **`write_repository`**
3. 设置过期时间 → **Create** → 复制 `glpat-...`

### Gitee（码云）

1. 头像 → **设置 → 私人令牌 → 生成新令牌**
2. 勾选 **`projects`**（仓库读写）
3. 复制令牌

> **用户名**字段一般留空即可（默认 `x-access-token`，GitHub 适用）；仓库地址用 HTTPS 形式，如 `https://github.com/你的用户名/你的私有仓库.git`。

## 配置步骤

1. 打开 **设置 → 云同步**。
2. 填写：
   - **仓库地址**：HTTPS 形式，如 `https://github.com/yourname/ai-agent-config.git`
   - **分支**：默认 `main`
   - **访问令牌 (PAT)**：你的 Personal Access Token
   - **用户名**（可选，默认 `x-access-token`）、**设备名**（可选，显示在提交信息里）
3. 点击 **测试连接** 确认可达，再点 **保存**。
4. 在 **要同步的资产** 区域按需勾选/取消各项 agent 资产。

## 日常使用

- **推送（↑）**：把本机当前资产提交并推送到远端。
  - 若远端有本机没有的提交（**分叉**），会弹窗让你选择：
    - **拉取**：采用远端版本（覆盖本地）
    - **强制推送**：用本地版本覆盖远端
- **拉取（↓）**：把远端内容拉取到本机并恢复（远端权威）。
  - 供应商 / MCP / Skills 会整体恢复；
  - agent 资产为**加性恢复**（远端没有的本地文件不会被删除）。

> 同步为**手动触发**，不会在后台自动推送。

## 换新电脑

1. 安装本应用与 git，打开 **设置 → 云同步**，填入相同的仓库地址与 PAT，保存。
2. 点击 **拉取**，即可恢复供应商、MCP、Skills 以及各工具的 agent 配置。

## 从 WebDAV 迁移

新版本已**移除 WebDAV 同步**，改用 Git。迁移步骤：

1. 升级前（旧版本）：用旧版 WebDAV 把数据同步一次，或用 **设置 → 数据 → 导出配置** 导出一份 `.sql` 备份，留底。
2. 升级到新版本后：旧的 WebDAV 设置会被忽略，本机数据（供应商 / MCP / Skills）保持不变。
3. 按上文「配置步骤」配好 Git 仓库，点 **推送**，即可把本机资产作为新基线提交到 Git 仓库。
4. 其它机器升级后，配好同一个仓库并 **拉取** 即可。

> 如果升级后发现本机数据异常，可用 **设置 → 数据 → 导入配置** 导入第 1 步留底的 `.sql` 备份恢复。

## 注意事项与限制

- **密钥明文**：仓库必须私有；如需更高安全性，请在仓库托管侧启用访问控制/加密。
- **需要 git**：传输层调用系统 `git`，PAT 通过 `GIT_ASKPASS` 注入，不会出现在命令行参数或 reflog 中。
- **跨操作系统的绝对路径**：`settings.json` / hooks 里若写了绝对路径或特定 shell 命令，恢复是逐字照搬，Windows ↔ macOS 之间可能需要手动调整。
- **默认不同步**：会话记录（`~/.claude/projects/`）、日志、缓存、`.git`、`.DS_Store`、`node_modules` 等不会进入仓库。
- **删除不传播**（agent 资产）：拉取时若远端缺少某资产，本机对应文件会被保留而非删除。
