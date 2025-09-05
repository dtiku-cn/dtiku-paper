FROM postgres:15.8

RUN apt-get update && apt-get install -y curl

RUN curl -fsSL https://repo.pigsty.io/pig | bash

RUN pig repo add pigsty pgdg -u && \
    pig ext install pgvector -v 15 && \
    pig ext install pg_mooncake -v 15 \
    pig ext install pg_partman -v 15