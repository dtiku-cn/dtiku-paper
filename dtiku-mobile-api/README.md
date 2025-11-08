# dtiku-mobile-api

移动端 API 服务,为移动应用提供独立的 REST API 接口。

## 功能特性

- **用户管理**: 注册、登录、用户信息查询
- **试卷管理**: 试卷列表、试卷详情、试卷聚类
- **题目管理**: 题目搜索、题目详情、题目推荐
- **成语管理**: 成语列表、成语详情
- **帖子管理**: 帖子列表、帖子详情、帖子创建/编辑/删除
- **支付管理**: 订单创建、订单查询
- **系统配置**: 系统配置查询

## 技术栈

- **框架**: Spring-RS (基于 Axum)
- **数据库**: PostgreSQL + SeaORM
- **缓存**: Redis
- **监控**: OpenTelemetry
- **支付**: 支付宝/微信支付

## 开发

### 本地运行

```bash
# 设置环境变量
export DATABASE_URL="postgres://postgres:12345@localhost:5432/empty_tiku"
export REDIS_URL="redis://localhost"
export JWT_SECRET="your-secret-key"

# 运行开发服务器
cargo run --bin mobile-api
```

服务默认运行在 `http://localhost:18088`

### 配置文件

- `config/app.toml`: 开发环境配置
- `config/app-prod.toml`: 生产环境配置

## API 文档

### 用户 API

- `POST /api/user/login` - 用户登录
- `POST /api/user/register` - 用户注册
- `GET /api/user/info` - 获取用户信息
- `POST /api/user/logout` - 用户登出

### 试卷 API

- `GET /api/paper/list` - 获取试卷列表
- `GET /api/paper/{id}` - 获取试卷详情
- `GET /api/paper/cluster` - 获取试卷聚类信息

### 题目 API

- `GET /api/question/search` - 搜索题目
- `GET /api/question/{id}` - 获取题目详情
- `GET /api/question/recommend` - 获取推荐题目
- `GET /api/question/section` - 获取章节题目

### 成语 API

- `GET /api/idiom/list` - 获取成语列表
- `GET /api/idiom/{id}` - 获取成语详情

### 帖子 API

- `GET /api/issue/list` - 获取帖子列表
- `GET /api/issue/{id}` - 获取帖子详情
- `POST /api/issue/create` - 创建帖子
- `PUT /api/issue/{id}/update` - 更新帖子
- `DELETE /api/issue/{id}/delete` - 删除帖子

### 支付 API

- `POST /api/pay/create` - 创建支付订单
- `GET /api/pay/query/{id}` - 查询订单状态

### 系统 API

- `GET /api/system/config` - 获取系统配置

## 部署

### Docker 部署

```bash
# 构建镜像
docker build -f mobile-api.Dockerfile -t dtiku-mobile-api .

# 运行容器
docker run -d \
  -p 18088:18088 \
  -e DATABASE_URL="postgres://..." \
  -e REDIS_URL="redis://..." \
  -e JWT_SECRET="..." \
  dtiku-mobile-api
```

## 限流

API 使用 Governor 进行限流:
- 平均速率: 10 req/s
- 突发限制: 30 req

## 认证

API 使用 JWT 进行认证,token 通过 cookie 传递。部分接口需要认证:
- 用户信息查询
- 帖子创建/编辑/删除
- 支付订单创建/查询

## 错误处理

所有错误响应统一格式:

```json
{
  "error": true,
  "status": 404,
  "message": "Resource not found"
}
```

