------- arroyo是flink替代品，用于流式计算 -------
-- nginx访问日志
CREATE TABLE nginx_access_log (
    remote_addr TEXT,
    remote_user TEXT,
    time_local TEXT,
    timestamp TEXT,
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
    format = 'json'
);

-- redis中的活跃用户
CREATE TABLE users (
    record_key TEXT METADATA FROM 'key' PRIMARY KEY,
    id INT,
    name TEXT,
    expired TIMESTAMP
) with (
    connector = 'redis',
    address = 'redis://redis:6379/0',
    format = 'json',
    type = 'lookup',
    'json.timestamp_format' = 'RFC3339',
    'lookup.cache.max_bytes' = 1000000,
    'lookup.cache.ttl' = interval '5 seconds'
);

CREATE TABLE redis_block_ip_list (
    ip TEXT,
    timestamp TIMESTAMP
) WITH (  
    connector = 'redis',
    address = 'redis://redis:6379/0',  
    format = 'json',  
    type = 'sink',  
    target = 'list',
    'target.key_prefix' = 'block_ip:',  
    'target.max_length' = 1000,  -- 可选,限制列表最大长度
    'target.operation' = 'append'  -- 或 'prepend'
);

-- 统计每个 IP 的访问次数（每1分钟窗口）  
SELECT   
    remote_addr,
    COUNT(*) as request_count,  
    HOP(interval '1 second', interval '1 minute') as window  
FROM nginx_access_log  
GROUP BY remote_addr, window
HAVING request_count>1;

-- 统计每个 用户 的访问次数（每1分钟窗口）
SELECT
    remote_user,
    COUNT(*) as request_count,  
    HOP(interval '1 second', interval '1 minute') as window  
FROM nginx_access_log
WHERE remote_user != '-'
GROUP BY remote_user, window
HAVING request_count>1;

SELECT   
    t.user_key,  
    t.request_count,  
    t.window,  
    u.id,  
    u.name,  
    u.expired  
FROM (  
    SELECT  
        REPLACE(remote_user, 'u:', 'user:') as user_key,  
        COUNT(*) as request_count,    
        tumble(interval '1 minute') as window    
    FROM nginx_access_log  
    WHERE remote_user like 'u:%'
    GROUP BY remote_user, window
    HAVING request_count > 1  
) t   
LEFT JOIN users u ON t.user_key = u.record_key;
