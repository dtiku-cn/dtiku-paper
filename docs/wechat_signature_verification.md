# 微信公众号签名验证实现

## 概述

本文档说明微信公众号消息推送的签名验证实现，确保接收到的消息确实来自微信服务器。

## 签名验证算法

根据[微信官方文档](https://developers.weixin.qq.com/doc/service/guide/dev/push/)，签名验证算法如下：

### 步骤

1. **获取参数**：从微信请求中获取 `signature`、`timestamp`、`nonce` 三个参数
2. **字典序排序**：将 `token`、`timestamp`、`nonce` 三个参数进行字典序排序
3. **拼接字符串**：将排序后的三个参数拼接成一个字符串
4. **SHA1 加密**：对拼接后的字符串进行 SHA1 加密
5. **比较签名**：将加密结果与微信提供的 `signature` 进行比较

### 实现代码

```rust
use sha1::Digest;

fn verify_wechat_signature(
    token: &str,
    params: &std::collections::HashMap<String, String>,
) -> bool {
    // 如果 token 为空，跳过验证（开发环境）
    if token.is_empty() {
        return true;
    }
    
    // 获取签名参数
    let signature = match params.get("signature") {
        Some(s) => s,
        None => return false,
    };
    
    let timestamp = params.get("timestamp").map(|s| s.as_str()).unwrap_or("");
    let nonce = params.get("nonce").map(|s| s.as_str()).unwrap_or("");
    
    // 将 token、timestamp、nonce 进行字典序排序
    let mut arr = vec![token, timestamp, nonce];
    arr.sort();
    
    // 拼接成字符串
    let concatenated = arr.join("");
    
    // 进行 SHA1 加密
    let mut hasher = sha1::Sha1::new();
    hasher.update(concatenated.as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);
    
    // 比较签名
    hash_hex == *signature
}
```

## 两种请求类型

### 1. 接入验证（GET 请求）

当在微信公众平台配置服务器时，微信会发送 GET 请求验证服务器：

**请求参数**：
- `signature`: 微信加密签名
- `timestamp`: 时间戳
- `nonce`: 随机数
- `echostr`: 随机字符串

**处理流程**：
1. 验证签名
2. 签名验证成功后，原样返回 `echostr` 参数内容

**路由**：`GET /api/wechat/callback`

```rust
#[get("/api/wechat/callback")]
async fn wechat_verify_callback(
    RawQuery(query): RawQuery,
    Component(auth_config): Component<AuthConfig>,
) -> Result<impl IntoResponse> {
    let params: HashMap<String, String> = 
        serde_urlencoded::from_str(&query.unwrap_or_default())?;
    
    // 验证签名
    if !verify_wechat_signature(&auth_config.wechat_mp_event_token, &params) {
        return Err(KnownWebError::unauthorized("签名验证失败").into());
    }
    
    // 返回 echostr
    let echostr = params.get("echostr")
        .ok_or_else(|| anyhow::anyhow!("缺少 echostr 参数"))?;
    
    Ok(echostr.clone())
}
```

### 2. 事件推送（POST 请求）

用户与公众号交互时（如扫码、关注），微信会发送 POST 请求推送事件：

**请求参数**（Query String）：
- `signature`: 微信加密签名
- `timestamp`: 时间戳
- `nonce`: 随机数
- `openid`: 用户 openid（可选）
- `encrypt_type`: 加密类型（可选）
- `msg_signature`: 消息签名（加密模式）

**请求体**：XML 格式的消息内容

**处理流程**：
1. 验证签名
2. 解析 XML 消息
3. 处理业务逻辑
4. 返回 "success" 字符串

**路由**：`POST /api/wechat/callback`

```rust
#[post("/api/wechat/callback")]
async fn wechat_event_callback(
    RawQuery(query): RawQuery,
    Component(redis): Component<Redis>,
    Component(us): Component<UserService>,
    Component(auth_config): Component<AuthConfig>,
    body: Bytes,
) -> Result<impl IntoResponse> {
    let params: HashMap<String, String> = 
        serde_urlencoded::from_str(&query.unwrap_or_default())?;
    
    // 验证签名
    if !verify_wechat_signature(&auth_config.wechat_mp_event_token, &params) {
        return Err(KnownWebError::unauthorized("签名验证失败").into());
    }
    
    // 解析 XML 并处理业务逻辑
    let body_str = String::from_utf8(body.to_vec())?;
    let msg: WechatEventMessage = quick_xml::de::from_str(&body_str)?;
    
    // ... 处理业务逻辑 ...
    
    Ok("success".to_string())
}
```

## 配置

### 环境变量

```bash
# 微信公众号配置
WECHAT_MP_APP_ID=你的AppID
WECHAT_MP_APP_SECRET=你的AppSecret
WECHAT_MP_EVENT_TOKEN=你的Token          # 用于签名验证
WECHAT_MP_EVENT_ENCODING_AES_KEY=你的Key  # 用于消息加解密（可选）
```

### 配置文件

`dtiku-web/config/app.toml`:

```toml
[auth]
wechat_mp_app_id = "${WECHAT_MP_APP_ID}"
wechat_mp_app_secret = "${WECHAT_MP_APP_SECRET}"
wechat_mp_event_token = "${WECHAT_MP_EVENT_TOKEN}"
wechat_mp_event_encoding_aes_key = "${WECHAT_MP_EVENT_ENCODING_AES_KEY}"
```

## 微信公众平台配置

1. 登录 [微信公众平台](https://mp.weixin.qq.com)
2. 进入 **开发** -> **基本配置**
3. 配置服务器信息：
   - **URL**: `https://your-domain.com/api/wechat/callback`
   - **Token**: 与 `WECHAT_MP_EVENT_TOKEN` 保持一致
   - **EncodingAESKey**: 点击随机生成
   - **消息加解密方式**: 
     - 明文模式（开发推荐）
     - 兼容模式
     - 安全模式（生产推荐）

4. 点击 **提交**
   - 微信会发送 GET 请求验证
   - 验证成功后才能启用配置

## 安全说明

### 已实现的安全措施

1. ✅ **签名验证**：所有请求都会验证签名，确保来自微信服务器
2. ✅ **SHA1 加密**：使用 SHA1 算法进行签名计算
3. ✅ **字典序排序**：严格按照微信要求的算法实现
4. ✅ **详细日志**：签名验证失败时记录详细信息便于调试

### 开发模式

如果 `WECHAT_MP_EVENT_TOKEN` 环境变量为空：
- 会跳过签名验证
- 记录警告日志
- **仅用于本地开发，生产环境必须配置**

### 生产环境建议

1. **必须配置 Token**：不要在生产环境跳过签名验证
2. **使用 HTTPS**：微信要求服务器必须使用 HTTPS
3. **启用加密模式**：对于敏感业务，建议使用安全模式
4. **监控日志**：定期检查是否有签名验证失败的记录

## 故障排查

### 签名验证失败

如果签名验证失败，检查：

1. **Token 配置**：确保环境变量 `WECHAT_MP_EVENT_TOKEN` 与微信公众平台配置一致
2. **URL 配置**：确保微信公众平台配置的 URL 正确
3. **时间同步**：确保服务器时间正确（影响 timestamp 参数）
4. **日志信息**：查看日志中的详细信息

```bash
# 日志示例
WARN 签名验证失败: 计算=abc123..., 收到=xyz789..., token=mytoken, timestamp=1234567890, nonce=abc123
```

### 接入验证失败

1. **检查路由**：确保 GET 请求能访问 `/api/wechat/callback`
2. **检查返回值**：必须原样返回 `echostr` 参数
3. **检查签名**：使用上述算法验证本地计算的签名是否正确

## 参考文档

- [微信公众号消息推送](https://developers.weixin.qq.com/doc/service/guide/dev/push/)
- [微信公众号消息加解密](https://developers.weixin.qq.com/doc/service/guide/dev/push/encryption.html)

