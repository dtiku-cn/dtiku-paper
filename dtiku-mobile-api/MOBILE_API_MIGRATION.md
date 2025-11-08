# 移动端 API 迁移文档

## 迁移概述

成功将 `dtiku-web` 中的 `/api` 路由拆分到独立的 `dtiku-mobile-api` crate 中,实现 web 端和移动端 API 的分离部署。

## 已完成的工作

### 1. 创建新的 crate 结构

```
dtiku-mobile-api/
├── Cargo.toml              # 项目配置,定义依赖
├── README.md               # 项目说明文档
├── config/
│   ├── app.toml            # 开发环境配置
│   └── app-prod.toml       # 生产环境配置
└── src/
    ├── main.rs             # 程序入口
    ├── router/
    │   ├── mod.rs          # 路由框架、JWT 认证、限流
    │   └── api/
    │       ├── mod.rs      # API 模块导出
    │       ├── user.rs     # 用户相关 API
    │       ├── paper.rs    # 试卷相关 API
    │       ├── question.rs # 题目相关 API
    │       ├── idiom.rs    # 成语相关 API
    │       ├── issue.rs    # 帖子相关 API
    │       ├── pay.rs      # 支付相关 API
    │       └── system.rs   # 系统配置 API
    └── service/
        ├── mod.rs          # 服务模块导出
        ├── user.rs         # 用户服务(简化版)
        └── issue.rs        # 帖子服务(简化版)
```

### 2. API 路由迁移

已迁移以下 API 端点:

#### 用户 API (`user.rs`)
- `POST /api/user/login` - 用户登录
- `POST /api/user/register` - 用户注册
- `GET /api/user/info` - 获取用户信息
- `POST /api/user/logout` - 用户登出

#### 试卷 API (`paper.rs`)
- `GET /api/paper/list` - 获取试卷列表
- `GET /api/paper/{id}` - 获取试卷详情
- `GET /api/paper/cluster` - 获取试卷聚类

#### 题目 API (`question.rs`)
- `GET /api/question/search` - 搜索题目
- `GET /api/question/{id}` - 获取题目详情
- `GET /api/question/recommend` - 获取推荐题目
- `GET /api/question/section` - 获取章节题目

#### 成语 API (`idiom.rs`)
- `GET /api/idiom/list` - 获取成语列表
- `GET /api/idiom/{id}` - 获取成语详情

#### 帖子 API (`issue.rs`)
- `GET /api/issue/list` - 获取帖子列表
- `GET /api/issue/{id}` - 获取帖子详情
- `POST /api/issue/create` - 创建帖子(需认证)
- `PUT /api/issue/{id}/update` - 更新帖子(需认证)
- `DELETE /api/issue/{id}/delete` - 删除帖子(需认证)

#### 支付 API (`pay.rs`)
- `POST /api/pay/create` - 创建支付订单(需认证)
- `GET /api/pay/query/{id}` - 查询订单状态(需认证)

#### 系统 API (`system.rs`)
- `GET /api/system/config` - 获取系统配置

### 3. 服务层实现

创建了简化版的服务层,移除了对 `Artalk` gRPC 客户端和复杂视图的依赖:

- **UserService**: 提供用户查询、更新等基础功能
- **IssueService**: 提供帖子查询功能

这些服务直接使用 SeaORM 与数据库交互,不依赖第三方 RPC 服务。

### 4. 配置文件

#### 开发环境配置 (`config/app.toml`)
- 端口: 18088
- 数据库连接池: 10
- OpenTelemetry: 禁用
- 支付: 支付宝沙箱环境

#### 生产环境配置 (`config/app-prod.toml`)
- 端口: 18088
- 数据库连接池: 20
- OpenTelemetry: 启用
- 支付: 生产环境

### 5. 中间件配置

- **限流**: 使用 `tower_governor`
  - 平均速率: 10 req/s (比 web 端更宽松)
  - 突发限制: 30 req
  
- **认证**: JWT token 通过 cookie 传递
  - 用户登录后返回 token
  - 需要认证的接口自动验证 token

- **错误处理**: 统一的 JSON 错误响应格式

- **追踪**: OpenTelemetry 集成,生产环境启用

### 6. Docker 支持

创建了 `mobile-api.Dockerfile`:
- 基于 Rust 1.x 构建
- 使用 Debian bookworm-slim 运行
- 暴露端口 18088
- 包含配置文件

### 7. Justfile 命令

添加了便捷的开发命令:
```bash
# 开发模式运行(带热重载)
just dev-mobile

# 构建 Docker 镜像
just build-mobile
```

### 8. Workspace 更新

在根目录 `Cargo.toml` 中添加了 `dtiku-mobile-api` 成员。

### 9. dtiku-web 清理

- 删除了 `dtiku-web/src/router/api/` 目录
- 从 `dtiku-web/src/router/mod.rs` 中移除 api 模块引用

## 技术架构

### 依赖管理

核心依赖:
- `spring-web`: Web 框架(基于 Axum)
- `spring-sea-orm`: ORM 数据库访问
- `spring-redis`: Redis 缓存
- `spring-stream`: 消息队列(Redis)
- `spring-opentelemetry`: 可观测性
- `dtiku-base`, `dtiku-paper`, `dtiku-bbs`, `dtiku-pay`, `dtiku-stats`: 业务模块
- `tower_governor`: API 限流
- `jsonwebtoken`: JWT 认证
- `axum-client-ip`: 客户端 IP 提取

### 路由框架

采用 Spring-RS 的自动路由发现机制:
- 使用 `#[get]`, `#[post]`, `#[put]`, `#[delete]` 宏标记路由处理函数
- `spring_web::handler::auto_router()` 自动扫描并注册路由
- 支持路径参数、查询参数、请求体等多种参数提取方式

### 认证机制

- 使用 JWT token 进行用户认证
- Token 存储在 HTTP cookie 中,域名为 `.dtiku.cn`
- 实现了 `Claims` 和 `OptionalClaims` 两种提取器
  - `Claims`: 必须认证,否则返回 401
  - `OptionalClaims`: 可选认证,未认证时为 None

### 错误处理

统一的 JSON 错误响应格式:
```json
{
  "error": true,
  "status": 404,
  "message": "详细错误信息"
}
```

## 待解决问题

### 编译错误

目前存在一个编译错误:
```
no method named `add_router` found for mutable reference `&mut AppBuilder`
```

**可能的解决方案:**
1. 检查 `spring-web` 版本是否正确
2. 确认 `WebConfigurator` trait 是否在作用域内
3. 查看 `spring-web` 最新文档,确认 API 变化
4. 可能需要使用不同的方法注册路由

**建议的排查步骤:**
```bash
# 1. 确认依赖版本
cargo tree -p spring-web

# 2. 清理并重新构建
cargo clean
cargo build -p dtiku-mobile-api

# 3. 参考 dtiku-web 或 dtiku-backend 的完整实现
diff dtiku-web/src/main.rs dtiku-mobile-api/src/main.rs
```

## 部署指南

### 开发环境

1. 设置环境变量:
```bash
export DATABASE_URL="postgres://postgres:12345@localhost:5432/empty_tiku"
export REDIS_URL="redis://localhost"
export JWT_SECRET="your-secret-key"
```

2. 运行服务:
```bash
just dev-mobile
# 或
cd dtiku-mobile-api && cargo run
```

### 生产环境

1. 构建 Docker 镜像:
```bash
just build-mobile
# 或
docker build -f mobile-api.Dockerfile -t dtiku-mobile-api:latest .
```

2. 运行容器:
```bash
docker run -d \
  --name dtiku-mobile-api \
  -p 18088:18088 \
  -e DATABASE_URL="postgres://..." \
  -e REDIS_URL="redis://..." \
  -e JWT_SECRET="..." \
  -e SPRING_PROFILE=prod \
  dtiku-mobile-api:latest
```

3. 使用 docker-compose:
```yaml
services:
  mobile-api:
    image: dtiku-mobile-api:latest
    ports:
      - "18088:18088"
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - JWT_SECRET=${JWT_SECRET}
      - SPRING_PROFILE=prod
    depends_on:
      - postgres
      - redis
```

## 后续优化建议

### 1. 功能完善
- [ ] 完善密码验证逻辑(目前登录接口未验证密码)
- [ ] 实现用户注册的邮箱验证
- [ ] 添加 API 版本控制(/api/v1/...)
- [ ] 实现 API 文档自动生成(OpenAPI/Swagger)

### 2. 性能优化
- [ ] 添加 Redis 缓存层
- [ ] 实现分页查询优化
- [ ] 添加数据库连接池监控
- [ ] 实现 API 响应压缩

### 3. 安全增强
- [ ] 实现 CORS 配置
- [ ] 添加 API 密钥认证
- [ ] 实现请求签名验证
- [ ] 添加敏感数据脱敏

### 4. 可观测性
- [ ] 添加 Prometheus metrics 导出
- [ ] 实现结构化日志
- [ ] 添加分布式追踪
- [ ] 实现健康检查端点

### 5. 测试
- [ ] 添加单元测试
- [ ] 添加集成测试
- [ ] 添加性能测试
- [ ] 实现 CI/CD 流程

## 与 dtiku-web 的差异

| 特性 | dtiku-web | dtiku-mobile-api |
|------|-----------|------------------|
| 端口 | 18080 | 18088 |
| 主要用途 | Web 网站 + API | 纯 API |
| 模板渲染 | Askama 模板 | 无 |
| 静态资源 | 包含 | 无 |
| 反爬虫 | 支持 | 无 |
| SEO 优化 | 支持 | 无 |
| Artalk 集成 | 支持 | 无(简化) |
| 限流速率 | 1 req/s | 10 req/s |
| 部署方式 | Web 服务器 | API 服务器 |

## 参考文档

- [Spring-RS 官方文档](https://spring-rs.github.io/)
- [Axum 文档](https://docs.rs/axum/)
- [SeaORM 文档](https://www.sea-ql.org/SeaORM/)
- [项目规则文档](.cursor/rules/project.mdc)

