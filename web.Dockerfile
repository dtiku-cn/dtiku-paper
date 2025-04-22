####################### rust builder
FROM rust:latest AS builder

COPY . /build

WORKDIR /build/dtiku-web

RUN cargo build --release

###################### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=builder /build/target/release/web ./app

COPY ./dtiku-web/config ./config
COPY ./dtiku-web/static ./static

EXPOSE 8080

ENTRYPOINT ["/runner/app"]