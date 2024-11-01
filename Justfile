## generate model
gen-model:
    sea-orm-cli generate entity --with-serde="both" --output-dir="dtiku-paper/src/model/_entities" --enum-extra-derives="strum::EnumString" --enum-extra-attributes="serde(rename_all = \"snake_case\")"

## dev-web
dev-web:
    cargo watch --workdir ./dtiku-web -x run

## dev-backend
dev-bk:
    cargo watch --workdir ./dtiku-backend -x run

## build backend
build-bk:
    docker build --tag holmofy/dtiku-backend:latest -f backend.Dockerfile .

## build web
build-web:
    docker build --tag holmofy/dtiku-web:latest -f web.Dockerfile .