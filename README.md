# QQBot-Relay

QQ Bot Webhook 到 WebSocket 的实时消息转发服务，基于 Rust + Axum 构建。

## 特性

- **消息缓存与重连补发**：自动缓存离线消息，客户端重连后补发（默认5分钟TTL）
- **双模式缓存**：公共缓存与 Token 私有缓存隔离
- **消息去重**：基于消息ID自动去重，避免重复推送
- **沙盒过滤**：支持按群组/成员/内容关键词过滤消息
- **速率控制**：补发时延迟启动 + 限速（防止瞬时压力过大）
- **失败熔断**：连续发送失败自动断开异常连接
- **Webhook 转发**：支持将收到的 Webhook 二次转发到多个目标
- **Ed25519 / HMAC-SHA256 签名验证**：兼容 QQ 开放平台握手，并保护 AppID 模式接口
- **配置热重载**：修改 `config.toml` 后自动生效，无需重启
- **内嵌 Web 面板**：`/web` 路径提供实时状态监控与管理功能
- **HTTPS 支持**：内置 TLS（rustls），也可配合反向代理使用

## 快速开始

### 环境要求

- Rust 1.80+
- Node.js 20+ 与 pnpm 10（用于构建内嵌 WebUI）
- 公网可访问的服务器（用于接收 QQ 开放平台的 Webhook 推送）

### 编译与运行

```bash
cargo build --release
./target/release/QQBot-Relay
```

首次运行会自动生成 `config.toml`，并在终端输出随机管理员初始密码。

### 配置

编辑 `config.toml`：

```toml
port = 8000

[ssl]
ssl_certfile = ""   # SSL 证书路径，留空则使用 HTTP
ssl_keyfile = ""    # SSL 私钥路径

[admin]
password = "首次启动时自动生成"
enabled = true
trust_proxy_headers = false # 仅在可信反向代理后开启

[cache]
max_public_messages = 1000
max_token_messages = 500
message_ttl = 300       # 消息缓存秒数
clean_interval = 120    # 缓存清理间隔

[webhook_forward]
timeout = 5
```

### 接入 QQ 开放平台

1. 启动服务后访问 `http://你的IP:8000/web`，使用管理员密码登录
2. 在面板中添加 AppID 和密钥
3. 复制验证链接填入 QQ 开放平台的 Webhook 回调地址
4. 复制 WebSocket 连接地址，客户端通过 WSS 连接即可接收消息

## API 端点

| 路径                       | 说明                         |
| -------------------------- | ---------------------------- |
| `POST /webhook?secret=xxx` | 接收 Webhook（密钥模式）     |
| `POST /api/{appid}`        | 接收 Webhook（签名 AppID 模式） |
| `GET /ws/{secret}`         | WebSocket 连接（密钥模式）   |
| `GET /api/ws/{appid}`      | WebSocket 连接（签名 AppID 模式） |
| `/web/`                    | 管理面板                     |

AppID 模式要求同时提供 `signature`、`timestamp` 和 `nonce`。签名内容为
`HMAC-SHA256(secret, timestamp + nonce + body)` 的十六进制字符串，时间戳与服务器时间偏差不能超过 5 分钟。QQ 平台首次 Ed25519 验证握手不要求该 HMAC。

## 关于 Issues

- 标题：简明描述问题
- 内容：复现步骤、期望结果、实际结果、截图、日志

## 关于 Pull Requests

- 遵循项目编码规范
- 确保 `cargo build` 和 `cargo clippy` 通过
