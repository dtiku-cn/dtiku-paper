# dtiku-mobile-api 下一步工作

## 当前状态

✅ 已完成:
- [x] 创建 `dtiku-mobile-api` crate 结构
- [x] 迁移所有 API 路由文件
- [x] 创建简化版服务层(UserService、IssueService)
- [x] 配置开发和生产环境文件
- [x] 创建 Dockerfile
- [x] 添加 Justfile 命令
- [x] 更新 workspace Cargo.toml
- [x] 从 dtiku-web 中移除 API 路由
- [x] 编写文档

⚠️ 待解决:
- [ ] 修复编译错误: `no method named 'add_router' found`

## 编译错误修复方案

### 问题诊断

错误信息:
```
no method named `add_router` found for mutable reference `&mut AppBuilder`
items from traits can only be used if the trait is in scope
```

### 可能的原因

1. **Spring-RS 版本问题**: `add_router` 方法可能在新版本中被重命名或移除
2. **Trait 导入问题**: `WebConfigurator` trait 可能未正确导入
3. **宏使用问题**: `#[auto_config]` 宏的使用方式可能不正确

### 解决步骤

#### 方案 1: 检查 Spring-RS 版本和文档

```bash
# 查看当前使用的 spring 版本
cargo tree -p spring -i spring

# 查看 spring-web 的文档
cargo doc --open -p spring-web

# 搜索 add_router 方法
rg "add_router" --type rust
```

#### 方案 2: 参考其他 crate 的实现

```bash
# 比较 dtiku-backend 和 dtiku-mobile-api 的差异
diff -u dtiku-backend/src/main.rs dtiku-mobile-api/src/main.rs
diff -u dtiku-backend/Cargo.toml dtiku-mobile-api/Cargo.toml
```

#### 方案 3: 尝试备选的路由注册方式

如果 `add_router` 确实被移除,可能需要使用新的 API:

```rust
// 可能的替代方案 1: 使用 configure_web
#[auto_config(WebConfigurator, StreamConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .opentelemetry_attrs([...])
        .configure_web(|web| {
            web.add_router(router::routers())
        })
        .add_plugin(...)
        .run()
        .await
}

// 可能的替代方案 2: 直接实现 WebConfigurator trait
impl WebConfigurator for MyApp {
    fn configure_router(&self, router: Router) -> Router {
        router.merge(router::routers())
    }
}

// 可能的替代方案 3: 使用更简单的方式
#[tokio::main]
async fn main() {
    let app_router = router::routers();
    
    App::new()
        .opentelemetry_attrs([...])
        .with_router(app_router) // 或类似的方法
        .add_plugin(...)
        .run()
        .await
}
```

#### 方案 4: 联系项目维护者

如果以上方案都不行,可以:
1. 查看 Spring-RS 的 GitHub Issues
2. 查看最新的示例代码
3. 提交 issue 询问

## 快速验证步骤

```bash
# 1. 清理构建缓存
cargo clean

# 2. 更新依赖
cargo update -p spring -p spring-web

# 3. 尝试编译
cargo build -p dtiku-mobile-api 2>&1 | tee build.log

# 4. 如果还是报错,查看 dtiku-web 是否能编译
cargo build -p dtiku-web

# 5. 如果 dtiku-web 也报错,说明是 workspace 级别的问题
# 如果 dtiku-web 正常,复制其 main.rs 的实现
```

## 临时解决方案

如果暂时无法解决编译问题,可以:

1. **回退到 dtiku-web**:
   ```bash
   git checkout dtiku-web/src/router/api
   git restore dtiku-web/src/router/mod.rs
   ```

2. **使用代理模式**:
   在 dtiku-web 中保留 API 路由,但让它代理到独立的 dtiku-mobile-api 服务。

3. **等待 Spring-RS 更新**:
   如果是框架的 bug,等待修复后再迁移。

## 验证清单

解决编译问题后,请完成以下验证:

### 编译验证
- [ ] `cargo check -p dtiku-mobile-api` 通过
- [ ] `cargo build -p dtiku-mobile-api --release` 通过
- [ ] `cargo clippy -p dtiku-mobile-api` 无警告

### 功能验证
- [ ] 服务能正常启动
- [ ] 健康检查通过
- [ ] 各 API 端点返回正确

### 测试用例

```bash
# 启动服务
export DATABASE_URL="postgres://postgres:12345@localhost:5432/empty_tiku"
export REDIS_URL="redis://localhost"
export JWT_SECRET="test-secret"
cargo run -p dtiku-mobile-api

# 测试系统配置 API
curl http://localhost:18088/api/system/config

# 测试试卷列表 API
curl http://localhost:18088/api/paper/list?page=1&page_size=10

# 测试用户注册(如果实现了)
curl -X POST http://localhost:18088/api/user/register \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"123456"}'

# 测试用户登录
curl -X POST http://localhost:18088/api/user/login \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"123456"}' \
  -c cookies.txt

# 测试需要认证的 API
curl http://localhost:18088/api/user/info \
  -b cookies.txt
```

### 性能验证
- [ ] 限流功能正常工作
- [ ] 数据库连接池正常
- [ ] 内存占用合理
- [ ] API 响应时间正常(<100ms)

### Docker 验证
```bash
# 构建镜像
docker build -f mobile-api.Dockerfile -t dtiku-mobile-api:test .

# 运行容器
docker run --rm \
  -p 18088:18088 \
  -e DATABASE_URL="..." \
  -e REDIS_URL="..." \
  -e JWT_SECRET="..." \
  dtiku-mobile-api:test

# 测试 API
curl http://localhost:18088/api/system/config
```

## 部署到生产环境

解决所有问题后:

1. **更新 CI/CD**:
   - 添加 dtiku-mobile-api 的构建步骤
   - 配置自动化测试
   - 设置镜像推送

2. **配置 Kubernetes/Docker Compose**:
   ```yaml
   services:
     mobile-api:
       image: dtiku-mobile-api:latest
       ports:
         - "18088:18088"
       environment:
         - SPRING_PROFILE=prod
         - DATABASE_URL=${DATABASE_URL}
         - REDIS_URL=${REDIS_URL}
         - JWT_SECRET=${JWT_SECRET}
       deploy:
         replicas: 2
         resources:
           limits:
             memory: 512M
           reservations:
             memory: 256M
   ```

3. **配置反向代理**:
   ```nginx
   # Nginx 配置
   upstream mobile_api {
       server mobile-api-1:18088;
       server mobile-api-2:18088;
   }

   server {
       listen 443 ssl;
       server_name api.dtiku.cn;

       location /api/ {
           proxy_pass http://mobile_api;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       }
   }
   ```

4. **监控设置**:
   - Prometheus metrics
   - Grafana dashboard
   - 告警规则

## 联系方式

如果遇到无法解决的问题:
- 查看项目文档: `.cursor/rules/project.mdc`
- 参考迁移文档: `MOBILE_API_MIGRATION.md`
- 查看 Spring-RS 文档: https://spring-rs.github.io/

