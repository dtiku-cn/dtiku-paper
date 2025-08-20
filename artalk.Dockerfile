####################### rust builder
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    &&\
    apt-get clean

COPY . /build

WORKDIR /build/dtiku-artalk

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/artalk ./dtiku-artalk

COPY ./dtiku-artalk/config ./config

EXPOSE 8080

ENTRYPOINT ["/runner/dtiku-artalk"]