# 实时流量监控后台页面

## 功能概述

基于 Arroyo 流处理引擎产生的实时统计数据，在管理后台新增了一个可视化监控页面，用于实时查看网站流量、安全防护、用户行为等关键指标。

## 实现内容

### 1. 后端 API (`dtiku-backend/src/router/stats.rs`)

提供 5 个 RESTful API 端点，从 Redis 读取 Arroyo 流处理产生的实时数据：

- **GET `/api/stats/blocked-ips`** - 获取 DDoS 防护封禁的 IP 列表
- **GET `/api/stats/suspicious-users`** - 获取异常用户行为监控列表
- **GET `/api/stats/traffic`** - 获取实时流量统计数据
- **GET `/api/stats/rate-limits`** - 获取智能限流配置
- **GET `/api/stats/hot-urls`** - 获取热门 URL 访问排行

所有 API 使用 Redis HGETALL 命令从固定的 Hash key 中读取数据，高效且不阻塞。

### 2. 前端监控页面 (`dtiku-backend/frontend/src/pages/RealtimeStats.tsx`)

完整的实时监控仪表盘，包含：

#### 统计卡片
- 总请求数
- 封禁 IP 数量
- 异常用户数量
- 热门 URL 数量

#### 可视化图表
- **状态码分布饼图** - 展示 2xx/3xx/4xx/5xx 的请求占比
- **Top 10 QPS 端点柱状图** - 显示 QPS 最高的 API 端点
- **Top 10 热门 URL 柱状图** - 显示访问量最高的 URL

#### 详细数据表格
1. **DDoS 防护表** - 显示被封禁的 IP、请求次数、封禁时间
2. **异常用户监控表** - 显示用户 ID、请求次数、错误率、风险等级
3. **智能限流配置表** - 显示各端点的当前 QPS、错误率、建议限流值
4. **热门 URL 分析表** - 显示 URL 路径、请求次数、4xx/5xx 错误统计

#### 响应式设计
- 支持移动端、平板、桌面端不同屏幕尺寸
- 表格列根据屏幕大小自适应显示/隐藏
- 图表尺寸和字体自动调整

#### 自动刷新
- 页面每 30 秒自动刷新数据
- 支持手动点击"刷新数据"按钮

### 3. 类型定义 (`dtiku-backend/frontend/src/types.ts`)

新增 TypeScript 接口定义：
- `BlockedIp` - 封禁 IP 数据结构
- `SuspiciousUser` - 异常用户数据结构
- `TrafficStats` - 流量统计数据结构
- `RateLimitConfig` - 限流配置数据结构
- `HotUrl` - 热门 URL 数据结构

### 4. API 服务层 (`dtiku-backend/frontend/src/services/api.ts`)

新增 `RealtimeStatsService` 服务，封装所有实时统计相关的 API 调用。

### 5. 路由配置 (`dtiku-backend/frontend/src/App.tsx`)

- 在左侧菜单添加"实时监控"菜单项（仪表盘图标）
- 注册路由 `/realtime-stats` 指向监控页面

## 使用方法

### 启动服务

1. **启动 Arroyo 流处理引擎**（参考 `sql/arroyo/README.md`）
   ```bash
   # 确保 Arroyo 已配置并运行流处理任务
   ```

2. **启动后端服务**
   ```bash
   cd dtiku-backend
   cargo run
   ```

3. **访问管理后台**
   ```
   打开浏览器访问: http://localhost:8080
   点击左侧菜单"实时监控"
   ```

### 数据流向

```
Nginx 访问日志 
  → Vector (采集) 
    → Arroyo (流处理) 
      → Redis (存储实时统计) 
        → 后端 API (读取) 
          → 前端页面 (展示)
```
