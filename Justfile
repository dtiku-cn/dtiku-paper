## generate model
gen-model:
    sea-orm-cli generate entity --with-serde="both" --output-dir="dtiku-paper/src/model/_entities" --enum-extra-derives="strum::EnumString" --enum-extra-attributes="serde(rename_all = \"snake_case\")"

## build backend
build-bk:
    docker build --tag holmofy/dtiku-backend:latest -f backend.Dockerfile .

## build web
build-web:
    docker build --tag holmofy/dtiku-web:latest -f web.Dockerfile .