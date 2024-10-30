FROM postgres:15.8

RUN apt-get update && apt-get install -y curl

RUN curl -sfL https://install.pgx.sh | sh -

RUN pgxman install pgvector