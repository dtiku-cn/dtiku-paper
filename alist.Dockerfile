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

WORKDIR /build/dtiku-alist

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/alist ./dtiku-alist

COPY ./dtiku-alist/config ./config

EXPOSE 8080

ENTRYPOINT ["/runner/dtiku-alist"]