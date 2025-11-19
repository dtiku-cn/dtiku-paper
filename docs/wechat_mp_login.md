# 微信公众号关注登录功能说明

## 功能概述

本功能实现了用户通过扫描二维码关注微信公众号来完成网站登录的流程。

## 技术实现

### 关键技术栈

- **HTTP 客户端**: `feignhttp` - 用于调用微信 API
- **XML 解析**: `quick-xml` - 使用 Serde API 解析微信事件推送的 XML
- **缓存**: Redis - 存储 access_token 和登录场景值
- **认证**: JWT (jsonwebtoken) - 生成用户登录 token

### 1. 核心流程

```
┌──────┐     ┌────────┐     ┌──────────┐     ┌────────┐
│ 前端 │────>│ 后端   │────>│ 微信服务器│────>│ 用户   │
└──────┘     └────────┘     └──────────┘     └────────┘
   │            │                │                 │
   │ 1.请求二维码 │                │                 │
   │───────────>│                │                 │
   │            │ 2.创建临时二维码 │                 │
   │            │───────────────>│                 │
   │            │ 3.返回ticket    │                 │
   │            │<───────────────│                 │
   │ 4.返回二维码  │                │                 │
   │<───────────│                │                 │
   │            │                │ 5.扫码关注       │
   │            │                │<────────────────│
   │            │ 6.事件推送       │                 │
   │            │<───────────────│                 │
   │ 7.轮询状态   │                │                 │
   │───────────>│                │                 │
   │ 8.返回token  │                │                 │
   │<───────────│                │                 │
```

### 2. API 接口

#### 2.1 生成登录二维码

**接口**: `GET /api/wechat/login/qrcode`

**响应**:
```json
{
  "scene_id": "uuid-string",
  "qrcode_url": "https://mp.weixin.qq.com/cgi-bin/showqrcode?ticket=xxx",
  "expire_seconds": 600
}
```

#### 2.2 查询登录状态

**接口**: `GET /api/wechat/login/status/:scene_id`

**响应**:
```json
{
  "status": "pending|success|expired",
  "token": "jwt-token",  // 仅在 status=success 时返回
  "user": {              // 仅在 status=success 时返回
    "id": 123,
    "name": "用户昵称",
    "avatar": "头像URL"
  }
}
```

#### 2.3 接收微信事件推送

**接口**: `POST /api/wechat/callback`

此接口用于接收微信服务器的事件推送，需在微信公众号后台配置。

**支持的事件类型**：

1. **subscribe 事件**（用户未关注时扫码）
   - 用户未关注公众号时扫描二维码
   - 用户先完成关注，然后推送此事件
   - EventKey 格式：`qrscene_场景值`

2. **SCAN 事件**（用户已关注时扫码）
   - 用户已关注公众号时扫描二维码
   - 直接推送此事件
   - EventKey 格式：`场景值`（不带 qrscene_ 前缀）

### 3. 配置说明

在 `dtiku-web/config/app.toml` 中添加以下配置：

```toml
[auth]
wechat_mp_app_id = "${WECHAT_MP_APP_ID}"
wechat_mp_app_secret = "${WECHAT_MP_APP_SECRET}"
wechat_mp_event_token = "${WECHAT_MP_EVENT_TOKEN}"
wechat_mp_event_encoding_aes_key = "${WECHAT_MP_EVENT_ENCODING_AES_KEY}"
```

对应的环境变量：
- `WECHAT_MP_APP_ID`: 微信公众号的 AppID
- `WECHAT_MP_APP_SECRET`: 微信公众号的 AppSecret
- `WECHAT_MP_EVENT_TOKEN`: 微信公众号服务器配置的 Token（用于验签）
- `WECHAT_MP_EVENT_ENCODING_AES_KEY`: 微信公众号的 EncodingAESKey（消息加解密，可选）

### 4. 微信公众号配置

1. 登录微信公众平台 (https://mp.weixin.qq.com)
2. 进入 **开发** -> **基本配置**
3. 设置服务器配置：
   - **URL**: `https://your-domain.com/api/wechat/callback`
   - **Token**: 自定义字符串（需与环境变量 `WECHAT_MP_EVENT_TOKEN` 一致）**← 用于签名验证**
   - **EncodingAESKey**: 点击随机生成（可选，用于消息加密）
   - **消息加解密方式**: 
     - `明文模式`: 不加密（推荐用于开发和测试）
     - `兼容模式`: 同时支持明文和加密
     - `安全模式`: 必须加密（生产环境推荐）

4. 点击 **提交** 按钮
   - 微信会发送 GET 请求到你的服务器验证配置
   - 服务器需要验证签名并返回 `echostr` 参数
   - ✅ 验证成功后，服务器配置才会生效

5. 启用服务器配置

### 5. Redis 键说明

- `wechat:mp:access_token`: 缓存微信公众号的 access_token
- `wechat:login:scene:{scene_id}`: 存储登录场景的状态
  - `pending`: 待扫码
  - `{user_id}`: 已登录（存储用户ID）

### 6. 数据库

用户信息存储在 `user_info` 表中，关键字段：
- `id`: 用户ID
- `wechat_id`: 微信 OpenID
- `name`: 用户昵称
- `avatar`: 用户头像
- `created`: 创建时间
- `expired`: 会员过期时间

## 前端集成示例

```javascript
// 1. 获取二维码
async function getQrcode() {
  const response = await fetch('/api/wechat/login/qrcode');
  const data = await response.json();
  
  // 显示二维码
  document.getElementById('qrcode').src = data.qrcode_url;
  
  // 开始轮询登录状态
  pollLoginStatus(data.scene_id);
}

// 2. 轮询登录状态
async function pollLoginStatus(sceneId) {
  const interval = setInterval(async () => {
    const response = await fetch(`/api/wechat/login/status/${sceneId}`);
    const data = await response.json();
    
    if (data.status === 'success') {
      clearInterval(interval);
      // 保存 token
      localStorage.setItem('token', data.token);
      // 跳转到首页或显示用户信息
      window.location.href = '/';
    } else if (data.status === 'expired') {
      clearInterval(interval);
      // 二维码已过期，提示用户刷新
      alert('二维码已过期，请刷新重试');
    }
  }, 2000); // 每2秒轮询一次
}

// 启动
getQrcode();
```

## 代码结构

```
dtiku-web/src/
├── rpc/
│   └── wechat.rs              # 微信 API HTTP 客户端
├── service/
│   └── user.rs                # 用户服务（新增微信相关方法）
└── router/
    └── user.rs                # 用户路由（新增三个微信登录接口）
```

## 核心模块说明

### 1. `rpc/wechat.rs`

提供了调用微信 API 的 HTTP 客户端方法：
- `get_access_token()`: 获取 access_token
- `create_qrcode()`: 创建带参数二维码
- `get_user_info()`: 获取用户基本信息

二维码创建辅助方法（`CreateQrcodeRequest`）：
- `new_temp_str()`: 创建临时二维码（字符串场景值，最长64字符）**← 登录功能使用**
- `new_temp_id()`: 创建临时二维码（整数场景值，32位非0整型）
- `new_permanent_str()`: 创建永久二维码（字符串场景值，最多10万个）
- `new_permanent_id()`: 创建永久二维码（整数场景值，最多10万个）

**场景值说明**：
- `scene_id` 和 `scene_str` 都是可选字段，但必须提供其中一个
- 临时二维码有效期最长30天（本实现使用10分钟）
- 永久二维码无过期时间，但数量有限制

### 2. `service/user.rs`

扩展了 UserService，新增方法：
- `get_wechat_access_token()`: 获取并缓存 access_token
- `create_wechat_login_qrcode()`: 创建登录二维码
- `get_wechat_user_info()`: 获取微信用户信息
- `find_or_create_wechat_user()`: 根据微信用户信息创建或更新本地用户

### 3. `router/user.rs`

新增路由处理器：
- `wechat_login_qrcode()`: 生成登录二维码
- `wechat_login_status()`: 查询登录状态
- `wechat_verify_callback()`: **GET 请求** - 微信接入验证
- `wechat_event_callback()`: **POST 请求** - 接收微信事件推送
  - 使用 `quick-xml` 解析 XML
  - 处理 `subscribe` 事件（未关注用户扫码）
  - 处理 `SCAN` 事件（已关注用户扫码）

工具函数：
- `verify_wechat_signature()`: 验证微信签名（SHA1）
- `handle_wechat_login()`: 公共登录逻辑处理函数

## 安全性考虑

1. **Token 缓存**: access_token 提前 5 分钟过期，确保不会使用过期 token
2. **场景值有效期**: 登录场景值在 Redis 中的有效期为 10 分钟
3. **JWT 认证**: 登录成功后返回 JWT token，有效期 30 天
4. **签名验证**: ✅ 已实现微信消息签名验证（SHA1）
   - 将 token、timestamp、nonce 进行字典序排序
   - 拼接后进行 SHA1 加密
   - 与微信提供的 signature 比对
   - 验证失败时拒绝请求
5. **开发模式**: 如果 `WECHAT_MP_EVENT_TOKEN` 为空，会跳过签名验证（仅用于开发）

## 注意事项

1. 需要一个已认证的微信公众号（服务号或订阅号）
2. 公众号需要有二维码生成权限
3. 服务器 URL 必须是公网可访问的 HTTPS 地址
4. 建议在生产环境中实现微信消息签名验证
5. 用户首次关注时会创建新用户，默认给予 7 天试用期

## 相关文档

- [微信公众平台技术文档](https://developers.weixin.qq.com/doc/offiaccount/Getting_Started/Overview.html)
- [生成带参数的二维码](https://developers.weixin.qq.com/doc/offiaccount/Account_Management/Generating_a_Parametric_QR_Code.html)
- [接收事件推送](https://developers.weixin.qq.com/doc/offiaccount/Message_Management/Receiving_event_pushes.html)

