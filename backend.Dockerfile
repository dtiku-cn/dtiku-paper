############### frontend builder
FROM node:20 as frontend_builder

WORKDIR /build

COPY frontend/package.json frontend/package-lock.json ./

# cache node_modules dependencies
RUN npm install

COPY frontend /build/

RUN npm run build

############### rust builder
FROM rust:latest AS builder

COPY . /build

WORKDIR /build/dtiku-backend

RUN cargo build --release

############### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=frontend_builder /build/build/ ./static

COPY --from=builder /build/target/release/backend ./app

COPY ./dtiku-backend/config ./config

EXPOSE 8080

ENTRYPOINT ["/runner/app"]