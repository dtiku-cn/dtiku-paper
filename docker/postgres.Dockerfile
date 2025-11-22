FROM debian:bookworm AS builder

RUN apt-get update && \  
    apt-get install -y \  
    build-essential \  
    cmake \  
    ninja-build \  
    libreadline-dev \  
    zlib1g-dev \  
    flex \  
    bison \  
    libxml2-dev \  
    libxslt1-dev \  
    libicu-dev \  
    libssl-dev \  
    libgeos-dev \  
    libproj-dev \  
    libgdal-dev \  
    libjson-c-dev \  
    libprotobuf-c-dev \  
    protobuf-c-compiler \  
    diffutils \  
    uuid-dev \  
    libossp-uuid-dev \  
    liblz4-dev \  
    liblzma-dev \  
    libsnappy-dev \  
    perl \  
    libtool \  
    libjansson-dev \  
    libcurl4-openssl-dev \  
    curl \  
    patch \  
    g++ \  
    libipc-run-perl \  
    wget \  
    git \  
    jq \  
    postgresql-server-dev-16 \  
    postgresql-16  
  
ENV VCPKG_VERSION=2025.01.13  
ENV PATH=/usr/lib/postgresql/16/bin:$PATH  
WORKDIR /build  
  
# Install vcpkg  
RUN git clone https://github.com/Microsoft/vcpkg.git -b ${VCPKG_VERSION} && \  
    ./vcpkg/bootstrap-vcpkg.sh && \  
    ./vcpkg/vcpkg install azure-identity-cpp azure-storage-blobs-cpp azure-storage-files-datalake-cpp openssl  
  
ENV VCPKG_TOOLCHAIN_PATH="/build/vcpkg/scripts/buildsystems/vcpkg.cmake"  

# Build pg_lake  
RUN git clone --recurse-submodules https://github.com/snowflake-labs/pg_lake.git && \  
    cd pg_lake/duckdb_pglake && make && make install && \  
    cd .. && make install-avro-local && make fast && make install-fast  
  
# Build pgduck_server  
RUN cd pg_lake/pgduck_server && make && make install

# ---------------------------
FROM postgres:16-bookworm

RUN apt-get update && apt-get install -y curl

## install pigsty
RUN curl -fSL https://repo.pigsty.io/pkg/pig/v0.6.1/pig_0.6.1-1_amd64.deb -o /tmp/pig_0.6.1-1_amd64.deb && \
    dpkg -i /tmp/pig_0.6.1-1_amd64.deb && \
    rm /tmp/pig_0.6.1-1_amd64.deb

RUN pig repo add pigsty pgdg -u && \
    pig ext install pgvector -v 16 && \
    pig ext install pg_mooncake -v 16 && \
    pig ext install pg_partman -v 16 && \
    pig ext install pg_repack -v 16 && \
    pig ext install pg_partman -v 16 && \ 
    echo "shared_preload_libraries = 'pg_extension_base'" >> /usr/share/postgresql/postgresql.conf.sample
