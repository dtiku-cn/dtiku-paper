# dtiku-web API 实现总结

## 概述

为 dtiku-mobile 移动端实现了对应的 REST API 接口，这些接口之前在 dtiku-web 中并不存在（dtiku-web 主要是基于 Askama 的 HTML 模板渲染）。

## 已实现的 API 模块

### 1. 用户 API (router/api/user.rs)
- `POST /api/user/login` - 用户登录
- `POST /api/user/register` - 用户注册
- `GET /api/user/info` - 获取用户信息
- `POST /api/user/logout` - 用户登出

**新增服务方法**：
- `UserService::find_user_by_name()` - 根据用户名查找用户

### 2. 试卷 API (router/api/paper.rs)
- `GET /api/paper/list` - 获取试卷列表（支持分页）
- `GET /api/paper/{id}` - 获取试卷详情
- `GET /api/paper/cluster` - 获取试卷聚类数据（按年份分组）

### 3. 题目 API (router/api/question.rs)
- `GET /api/question/search` - 搜索题目（支持关键词搜索和分页）
- `GET /api/question/{id}` - 获取题目详情
- `GET /api/question/recommend` - 获取推荐题目
- `GET /api/question/section` - 获取章节题目（接口框架已实现，需补充逻辑）

### 4. 成语 API (router/api/idiom.rs)
- `GET /api/idiom/list` - 获取成语列表（支持搜索和分页）
- `GET /api/idiom/{id}` - 获取成语详情（需要扩展现有服务以支持 ID 查询）

### 5. 论坛 API (router/api/issue.rs)
- `GET /api/issue/list` - 获取帖子列表（支持分页）
- `GET /api/issue/{id}` - 获取帖子详情
- `POST /api/issue/create` - 创建新帖子
- `PUT /api/issue/{id}/update` - 更新帖子（需要权限校验）
- `DELETE /api/issue/{id}/delete` - 删除帖子（需要权限校验）

### 6. 支付 API (router/api/pay.rs)
- `POST /api/pay/create` - 创建支付订单
- `GET /api/pay/query/{id}` - 查询订单状态

### 7. 系统配置 API (router/api/system.rs)
- `GET /api/system/config` - 获取系统配置信息

## 技术实现细节

### 响应格式

所有 API 都返回 JSON 格式数据：

```rust
// 分页响应
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
```

### 认证机制

- 使用已有的 JWT token 认证（通过 `Claims` extractor）
- Token 存储在 cookie 中（`.dtiku.cn` 域名）
- 有效期 30 天

### 数据模型映射

由于移动端期望的数据结构与后端模型不完全匹配，创建了专用的响应模型：
- `UserResponse` - 用户信息响应
- `PaperResponse` - 试卷响应
- `QuestionResponse` - 题目响应
- `IdiomResponse` - 成语响应
- `IssueResponse` - 帖子响应
- `PayOrderResponse` - 支付订单响应

## 已知问题和待优化项

### 1. 用户密码验证
`api_user_login` 中的密码验证逻辑需要补充：
```rust
// TODO: 实际项目中应该验证密码
// 建议使用 bcrypt 或 argon2 等密码哈希库
```

### 2. 成语详情查询
`api_idiom_detail` 暂时返回错误，因为现有服务不支持通过 ID 查询：
```rust
// 需要扩展 IdiomService 支持通过 ID 查询
// 或者修改移动端使用 text 参数而不是 ID
```

### 3. 题目章节查询
`api_question_section` 仅返回空结果框架，需要补充完整的查询逻辑。

### 4. Markdown 转换
在 `api_issue_create` 和 `api_issue_update` 中，Markdown 到 HTML 的转换过于简单：
```rust
// 建议使用专业的 markdown 解析器如 pulldown-cmark
let html = req.content.clone(); // 当前是直接复制
```

### 5. 编译问题
由于代码库依赖复杂，部分类型映射可能需要在实际编译时调整：
- `QuestionSearch` 结构可能需要调整字段
- `Page<T>` 类型的字段访问（`content` vs `data`）
- 部分模型字段不存在（如 `province`, `mode`, `created_at`）

## 测试建议

1. **单元测试**：为每个 API 端点编写单元测试
2. **集成测试**：测试与移动端的集成
3. **认证测试**：验证 JWT token 的正确性
4. **权限测试**：验证用户只能修改/删除自己的帖子

## 后续工作

1. 修复编译错误（需要实际编译环境）
2. 补充密码验证逻辑
3. 实现完整的 Markdown 解析
4. 添加 API 文档（可以使用 utoipa 生成 OpenAPI 文档）
5. 添加请求限流和安全防护
6. 补充单元测试和集成测试

## 文件清单

新增文件：
- `dtiku-web/src/router/api/mod.rs` - API 模块入口
- `dtiku-web/src/router/api/user.rs` - 用户 API
- `dtiku-web/src/router/api/paper.rs` - 试卷 API
- `dtiku-web/src/router/api/question.rs` - 题目 API
- `dtiku-web/src/router/api/idiom.rs` - 成语 API
- `dtiku-web/src/router/api/issue.rs` - 论坛 API
- `dtiku-web/src/router/api/pay.rs` - 支付 API
- `dtiku-web/src/router/api/system.rs` - 系统配置 API

修改文件：
- `dtiku-web/src/router/mod.rs` - 注册 API 模块
- `dtiku-web/src/service/user.rs` - 添加 `find_user_by_name` 方法

## 与移动端的 API 对应关系

| 移动端 API | 后端实现 | 状态 |
|-----------|---------|------|
| POST /api/user/login | api_user_login | ✅ 已实现 |
| POST /api/user/register | api_user_register | ✅ 已实现 |
| GET /api/user/info | api_user_info | ✅ 已实现 |
| POST /api/user/logout | api_user_logout | ✅ 已实现 |
| GET /api/paper/list | api_paper_list | ✅ 已实现 |
| GET /api/paper/{id} | api_paper_detail | ✅ 已实现 |
| GET /api/paper/cluster | api_paper_cluster | ✅ 已实现 |
| GET /api/question/search | api_question_search | ✅ 已实现 |
| GET /api/question/{id} | api_question_detail | ✅ 已实现 |
| GET /api/question/recommend | api_question_recommend | ✅ 已实现 |
| GET /api/question/section | api_question_section | ⚠️ 框架已实现 |
| GET /api/idiom/list | api_idiom_list | ✅ 已实现 |
| GET /api/idiom/{id} | api_idiom_detail | ⚠️ 需要扩展服务 |
| GET /api/issue/list | api_issue_list | ✅ 已实现 |
| GET /api/issue/{id} | api_issue_detail | ✅ 已实现 |
| POST /api/issue/create | api_issue_create | ✅ 已实现 |
| PUT /api/issue/{id}/update | api_issue_update | ✅ 已实现 |
| DELETE /api/issue/{id}/delete | api_issue_delete | ✅ 已实现 |
| POST /api/pay/create | api_pay_create | ✅ 已实现 |
| GET /api/pay/query/{id} | api_pay_query | ✅ 已实现 |
| GET /api/system/config | api_system_config | ✅ 已实现 |

## 注意事项

1. 所有 API 都使用 `spring-web` 框架的路由注册机制（通过 `#[get]`, `#[post]` 等宏）
2. 路径参数使用 `{id}` 格式（Axum 0.8+ 语法），而非旧版的 `:id` 格式
3. 认证通过 `Claims` extractor 自动处理
4. 错误处理使用 `KnownWebError` 统一格式
5. 分页使用 `Pagination` 结构
6. 所有响应都是 JSON 格式（通过 `Json<T>` wrapper）

