####################### rust builder
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    protobuf-compiler \
    libxml2 libxml2-dev \
    openssl libssl-dev ca-certificates \
    clang \
    build-essential \
    &&\
    apt-get clean

COPY . /build

WORKDIR /build/dtiku-mobile-api

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info
ENV TZ=Asia/Shanghai

WORKDIR /runner

COPY --from=builder /build/target/release/mobile-api ./dtiku-mobile-api

COPY ./dtiku-mobile-api/config ./config

EXPOSE 18088

ENTRYPOINT ["/runner/dtiku-mobile-api"]

