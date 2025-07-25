############### frontend builder
FROM node:20 as frontend_builder

WORKDIR /build

COPY dtiku-backend/frontend/package.json dtiku-backend/frontend/package-lock.json ./

# cache node_modules dependencies
RUN npm install

COPY dtiku-backend/frontend /build/

RUN npm run build

############### rust builder
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

WORKDIR /build/dtiku-backend

RUN cargo build --release

############### runner container
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libxml2 openssl libssl3 ca-certificates && apt-get clean

ENV RUST_LOG=info

WORKDIR /runner

COPY --from=frontend_builder /build/dist/ ./static

COPY --from=builder /build/target/release/backend ./dtiku-backend

COPY ./dtiku-backend/config ./config

EXPOSE 8000

ENTRYPOINT ["/runner/dtiku-backend"]