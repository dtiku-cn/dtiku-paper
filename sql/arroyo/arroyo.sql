------- arroyo是flink替代品，用于流式计算 -------
------- 实现目标：DDoS防护、账户安全、流量分析、智能限流 -------

-- ============ 数据源表 ============
-- nginx访问日志流（来自 Vector）
CREATE TABLE nginx_access_log (
    remote_addr TEXT,
    remote_user TEXT,
    time_local TEXT,
    timestamp TIMESTAMP,
    request TEXT,
    status INT,
    body_bytes_sent INT,
    http_referer TEXT,
    http_user_agent TEXT,
    http_x_forwarded_for TEXT,
    host TEXT,
    sent_http_content_type TEXT
) WITH (
    connector = 'websocket',
    endpoint = 'ws://vector',
    format = 'json',
    'json.timestamp_format' = 'RFC3339'
);

-- ============ 维度表（Lookup） ============
-- Redis中的活跃用户信息
CREATE TEMPORARY TABLE users (
    record_key TEXT METADATA FROM 'key' PRIMARY KEY,
    id INT,
    name TEXT,
    expired TIMESTAMP
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'lookup',
    'json.timestamp_format' = 'RFC3339',
    'lookup.cache.max_bytes' = 1000000,
    'lookup.cache.ttl' = interval '5 seconds'
);

-- ============ 输出表（Sink） ============
-- 1. DDoS防护：封禁IP黑名单
CREATE TABLE redis_block_ip_list (
    host TEXT NOT NULL,
    ip TEXT NOT NULL,
    request_count BIGINT,
    first_seen TIMESTAMP,
    last_seen TIMESTAMP,
    block_until TIMESTAMP
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'sink',
    target = 'hash',
    'target.key_prefix' = 'traffic:block_ip:',
    'target.field_column' = 'ip',
    'target.key_column' = 'host',
    'json.timestamp_format' = 'RFC3339'
);

-- 2. 账户安全：异常用户列表
CREATE TABLE redis_suspicious_users (
    host TEXT NOT NULL,
    user_id TEXT NOT NULL,
    user_name TEXT,
    request_count BIGINT,
    error_rate DOUBLE,
    window_start TIMESTAMP,
    window_end TIMESTAMP,
    risk_level TEXT  -- 'low', 'medium', 'high'
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'sink',
    target = 'hash',
    'target.key_prefix' = 'traffic:suspicious_user:',
    'target.field_column' = 'user_id',
    'target.key_column' = 'host',
    'json.timestamp_format' = 'RFC3339'
);

-- 3. 流量分析：实时访问统计（每分钟）
CREATE TABLE redis_traffic_stats (
    host TEXT NOT NULL,
    metric_type TEXT,  -- 'total', 'by_status', 'by_path'
    metric_key TEXT NOT NULL,
    value BIGINT,
    window_start TIMESTAMP,
    window_end TIMESTAMP
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'sink',
    target = 'hash',
    'target.key_prefix' = 'traffic:stats:',
    'target.field_column' = 'metric_key',
    'target.key_column' = 'host',
    'json.timestamp_format' = 'RFC3339'
);

-- 4. 智能限流：实时限流配置
CREATE TABLE redis_rate_limit_config (
    host TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    current_qps BIGINT,
    avg_response_time DOUBLE,
    error_rate DOUBLE,
    suggested_limit BIGINT,
    window_time TIMESTAMP
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'sink',
    target = 'hash',
    'target.key_prefix' = 'traffic:rate_limit:',
    'target.field_column' = 'endpoint',
    'target.key_column' = 'host',
    'json.timestamp_format' = 'RFC3339'
);

-- 5. URL热点分析
CREATE TABLE redis_hot_urls (
    host TEXT NOT NULL,
    url_path TEXT NOT NULL,
    request_count BIGINT,
    avg_response_size DOUBLE,
    status_4xx_count BIGINT,
    status_5xx_count BIGINT,
    window_start TIMESTAMP,
    window_end TIMESTAMP
) WITH (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'sink',
    target = 'hash',
    'target.key_prefix' = 'traffic:hot_url:',
    'target.field_column' = 'url_path',
    'target.key_column' = 'host',
    'json.timestamp_format' = 'RFC3339'
);

-- ============ 流处理任务（Pipelines） ============

-- ======== 任务1：DDoS防护 - 检测高频IP并自动封禁 ========
-- 检测规则：1分钟内请求超过100次的IP，或10秒内超过30次的IP
INSERT INTO redis_block_ip_list
SELECT 
    host,
    remote_addr as ip,
    COUNT(*) as request_count,
    MIN(timestamp) as first_seen,
    MAX(timestamp) as last_seen,
    MAX(timestamp) + INTERVAL '1 hour' as block_until
FROM nginx_access_log
WHERE 
    -- 排除轮询接口（健康检查、监控等）
    SPLIT_PART(request, ' ', 2) NOT LIKE '%/health%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/ping%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/metrics%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/heartbeat%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/alive%'
    -- 排除业务轮询接口
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/pay/%/status%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/api/wechat/login/status/%'
GROUP BY 
    host,
    remote_addr,
    HOP(INTERVAL '10 seconds', INTERVAL '1 minute')  -- 滑动窗口：每10秒滑动，窗口1分钟
HAVING COUNT(*) > 100  -- 1分钟内超过100次请求
   OR (COUNT(*) > 30 AND MAX(timestamp) - MIN(timestamp) < INTERVAL '10 seconds');  -- 10秒内超过30次

-- ======== 任务2：账户安全监控 - 识别异常用户行为 ========
-- 监控指标：高频请求、高错误率、异常访问模式
INSERT INTO redis_suspicious_users
SELECT
    t.host,
    COALESCE(u.id::TEXT, t.user_key) as user_id,
    COALESCE(u.name, 'Unknown') as user_name,
    t.request_count,
    t.error_rate,
    t.window_start,
    t.window_end,
    CASE
        WHEN t.request_count > 500 OR t.error_rate > 0.5 THEN 'high'
        WHEN t.request_count > 200 OR t.error_rate > 0.3 THEN 'medium'
        ELSE 'low'
    END as risk_level
FROM (
    SELECT
        host,
        REPLACE(remote_user, 'u:', 'user:') as user_key,
        COUNT(*) as request_count,
        SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END)::DOUBLE / COUNT(*)::DOUBLE as error_rate,
        MIN(timestamp) as window_start,
        MAX(timestamp) as window_end
    FROM nginx_access_log
    WHERE remote_user LIKE 'u:%'  -- 只统计已登录用户
    GROUP BY 
        host,
        remote_user,
        HOP(INTERVAL '30 seconds', INTERVAL '2 minutes')
    HAVING COUNT(*) > 100  -- 2分钟内超过100次请求
        OR SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END)::DOUBLE / COUNT(*)::DOUBLE > 0.2  -- 错误率超过20%
) t
LEFT JOIN users u ON t.user_key = u.record_key;

-- ======== 任务3：流量分析 - 总体流量统计 ========
-- 3.1 总请求数和流量
INSERT INTO redis_traffic_stats
SELECT
    host,
    'total_requests' as metric_type,
    'all' as metric_key,
    COUNT(*) as value,
    MIN(timestamp) as window_start,
    MAX(timestamp) as window_end
FROM nginx_access_log
GROUP BY 
    host,
    TUMBLE(INTERVAL '1 minute');

-- 3.2 按状态码分组统计
INSERT INTO redis_traffic_stats
SELECT
    host,
    'by_status' as metric_type,
    CASE
        WHEN status < 300 THEN '2xx'
        WHEN status < 400 THEN '3xx'
        WHEN status < 500 THEN '4xx'
        ELSE '5xx'
    END as metric_key,
    COUNT(*) as value,
    MIN(timestamp) as window_start,
    MAX(timestamp) as window_end
FROM nginx_access_log
GROUP BY 
    host,
    CASE
        WHEN status < 300 THEN '2xx'
        WHEN status < 400 THEN '3xx'
        WHEN status < 500 THEN '4xx'
        ELSE '5xx'
    END,
    TUMBLE(INTERVAL '1 minute');

-- ======== 任务4：智能限流 - 实时计算API端点限流配置 ========
-- 根据实时QPS、错误率动态调整限流阈值
INSERT INTO redis_rate_limit_config
SELECT
    host,
    url_path as endpoint,
    COUNT(*) / 60 as current_qps,  -- 每秒请求数（1分钟窗口）
    AVG(body_bytes_sent)::DOUBLE as avg_response_time,  -- 使用响应大小作为性能代理指标
    SUM(CASE WHEN status >= 500 THEN 1 ELSE 0 END)::DOUBLE / COUNT(*)::DOUBLE as error_rate,
    -- 智能限流算法：基于当前QPS和错误率计算建议限流值
    CASE
        WHEN SUM(CASE WHEN status >= 500 THEN 1 ELSE 0 END)::DOUBLE / COUNT(*)::DOUBLE > 0.1 
            THEN (COUNT(*) / 60) * 0.5  -- 错误率>10%，限流降至50%
        WHEN SUM(CASE WHEN status >= 500 THEN 1 ELSE 0 END)::DOUBLE / COUNT(*)::DOUBLE > 0.05 
            THEN (COUNT(*) / 60) * 0.7  -- 错误率>5%，限流降至70%
        ELSE (COUNT(*) / 60) * 1.5  -- 正常情况，允许150%的QPS
    END::BIGINT as suggested_limit,
    MAX(log_timestamp) as window_time
FROM (
    SELECT
        host,
        -- 提取URL路径（去除query参数）
        CASE
            WHEN POSITION('?' IN SPLIT_PART(request, ' ', 2)) > 0 
            THEN SUBSTRING(SPLIT_PART(request, ' ', 2), 1, POSITION('?' IN SPLIT_PART(request, ' ', 2)) - 1)
            ELSE SPLIT_PART(request, ' ', 2)
        END as url_path,
        body_bytes_sent,
        status,
        timestamp as log_timestamp
    FROM nginx_access_log
) subquery
WHERE url_path NOT LIKE '%/static/%'  -- 排除静态资源
GROUP BY 
    host,
    url_path,
    TUMBLE(INTERVAL '1 minute')
HAVING COUNT(*) > 10;  -- 只统计有一定流量的端点

-- ======== 任务5：URL热点分析 - Top热门访问路径 ========
INSERT INTO redis_hot_urls
SELECT
    host,
    url_path,
    COUNT(*) as request_count,
    AVG(body_bytes_sent)::DOUBLE as avg_response_size,
    SUM(CASE WHEN status >= 400 AND status < 500 THEN 1 ELSE 0 END) as status_4xx_count,
    SUM(CASE WHEN status >= 500 THEN 1 ELSE 0 END) as status_5xx_count,
    MIN(log_timestamp) as window_start,
    MAX(log_timestamp) as window_end
FROM (
    SELECT
        host,
        -- 规范化URL路径（聚合相似路径）
        CASE
            WHEN SPLIT_PART(request, ' ', 2) ~ '/paper/[0-9]+' THEN REGEXP_REPLACE(SPLIT_PART(request, ' ', 2), '/[0-9]+', '/:id')
            WHEN SPLIT_PART(request, ' ', 2) ~ '/user/[0-9]+' THEN REGEXP_REPLACE(SPLIT_PART(request, ' ', 2), '/[0-9]+', '/:id')
            WHEN SPLIT_PART(request, ' ', 2) ~ '/idiom/[^/]+$' THEN REGEXP_REPLACE(SPLIT_PART(request, ' ', 2), '/[^/]+$', '/:idiom')
            WHEN SPLIT_PART(request, ' ', 2) ~ '/word/[^/]+$' THEN REGEXP_REPLACE(SPLIT_PART(request, ' ', 2), '/[^/]+$', '/:word')
            WHEN POSITION('?' IN SPLIT_PART(request, ' ', 2)) > 0 
                THEN SUBSTRING(SPLIT_PART(request, ' ', 2), 1, POSITION('?' IN SPLIT_PART(request, ' ', 2)) - 1)
            ELSE SPLIT_PART(request, ' ', 2)
        END as url_path,
        body_bytes_sent,
        status,
        timestamp as log_timestamp
    FROM nginx_access_log
) parsed_logs
GROUP BY 
    host,
    url_path,
    TUMBLE(INTERVAL '5 minutes')
HAVING COUNT(*) > 5;  -- 过滤掉冷门URL

-- ======== 任务6：爬虫/机器人检测 ========
-- 检测规则：User-Agent特征 + 访问频率 + 无Referer
INSERT INTO redis_block_ip_list
SELECT
    host,
    remote_addr as ip,
    COUNT(*) as request_count,
    MIN(timestamp) as first_seen,
    MAX(timestamp) as last_seen,
    MAX(timestamp) + INTERVAL '2 hours' as block_until
FROM nginx_access_log
WHERE 
    -- 常见爬虫UA特征（非白名单爬虫）
    (http_user_agent LIKE '%bot%' 
     OR http_user_agent LIKE '%crawler%' 
     OR http_user_agent LIKE '%spider%'
     OR http_user_agent LIKE '%scraper%'
     OR http_user_agent = '-'
     OR http_user_agent = '')
    AND http_user_agent NOT LIKE '%Googlebot%'  -- 排除合法搜索引擎
    AND http_user_agent NOT LIKE '%Bingbot%'
    AND http_user_agent NOT LIKE '%Baidu%'
    AND http_user_agent NOT LIKE '%Sogou%'
    AND http_referer IN ('-', '')  -- 无来源页面
    -- 排除轮询接口（健康检查、监控等）
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/health%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/ping%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/metrics%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/heartbeat%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/alive%'
    -- 排除业务轮询接口
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/pay/%/status%'
    AND SPLIT_PART(request, ' ', 2) NOT LIKE '%/api/wechat/login/status/%'
GROUP BY 
    host,
    remote_addr,
    TUMBLE(INTERVAL '5 minutes')
HAVING COUNT(*) > 50;  -- 5分钟内超过50次请求的可疑爬虫
