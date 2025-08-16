FROM rust:1.81 as acme-build

ARG NGINX_VERSION=1.29.1
WORKDIR /build

# 安装 Nginx 构建依赖
RUN apt-get update && apt-get install -y \
    git curl build-essential libpcre3-dev zlib1g-dev libssl-dev clang pkg-config cmake \
    && rm -rf /var/lib/apt/lists/*

# 拉取 Nginx 源码 (仅用于构建)
RUN curl -fSL http://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz -o nginx-${NGINX_VERSION}.tar.gz \
    && tar -xzf nginx-${NGINX_VERSION}.tar.gz

# 拉取 nginx-acme
RUN git clone https://github.com/nginx/nginx-acme.git

# 构建 nginx-acme
WORKDIR /build/nginx-${NGINX_VERSION}
RUN auto/configure --with-compat --with-http_ssl_module --add-dynamic-module=/build/nginx-acme \
    && make modules

#---------------------------------------------------------------------
FROM nginx:1.29.1-otel

# 拷贝编译好的模块到最终镜像
COPY --from=acme-build /build/nginx-1.29.1/objs/ngx_http_acme_module.so /etc/nginx/modules/

# 默认加载模块
RUN echo "load_module modules/ngx_http_acme_module.so;" >> /etc/nginx/nginx.conf
