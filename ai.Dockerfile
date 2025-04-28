####################### rust builder
FROM rust:latest AS builder

COPY . /build

WORKDIR /build/dtiku-ai

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/ai ./dtiku-ai

COPY ./dtiku-web/config ./config

EXPOSE 9090

ENTRYPOINT ["/runner/dtiku-ai"]