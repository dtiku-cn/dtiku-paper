FROM postgres:15.8

RUN apt-get update && apt-get install -y curl

## install pigsty
RUN curl -fSL https://repo.pigsty.io/pkg/pig/v0.6.1/pig_0.6.1-1_amd64.deb -o /tmp/pig_0.6.1-1_amd64.deb && \
    dpkg -i /tmp/pig_0.6.1-1_amd64.deb && \
    rm /tmp/pig_0.6.1-1_amd64.deb

RUN pig repo add pigsty pgdg -u && \
    pig ext install pgvector -v 15 && \
    pig ext install pg_mooncake -v 15 && \
    pig ext install pg_partman -v 15 && \
    pig ext install pg_repack -v 15 && \
    pig ext install pg_partman -v 15