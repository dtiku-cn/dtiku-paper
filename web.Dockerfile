####################### rust builder
FROM rust:latest AS builder

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

WORKDIR /build/dtiku-web

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/web ./dtiku-web

COPY ./dtiku-web/config ./config
COPY ./dtiku-web/static ./static

EXPOSE 8080

ENTRYPOINT ["/runner/dtiku-web"]