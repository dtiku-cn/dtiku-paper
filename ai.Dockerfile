####################### rust builder
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    &&\
    apt-get clean

COPY . /build

WORKDIR /build/dtiku-ai

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

# 安装必要工具和依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    unzip \
    libomp5 \
    && rm -rf /var/lib/apt/lists/*

# 设置版本
ENV ORT_VERSION=1.21.0

# 下载、解压、安装并清理
RUN curl -sL https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-linux-x64-${ORT_VERSION}.tgz \
    -o /tmp/onnxruntime.tgz && \
    mkdir -p /tmp/ort && \
    tar -xzf /tmp/onnxruntime.tgz -C /tmp/ort --strip-components=1 && \
    cp -r /tmp/ort/lib/* /usr/local/lib/ && \
    cp -r /tmp/ort/include/* /usr/local/include/ && \
    ldconfig && \
    rm -rf /tmp/onnxruntime.tgz /tmp/ort

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/ai ./dtiku-ai

COPY ./dtiku-web/config ./config

EXPOSE 9090

ENTRYPOINT ["/runner/dtiku-ai"]