## generate model
gen-model:
    sea-orm-cli generate entity --with-serde="both" --output-dir="dtiku-paper/src/model/_entities" --enum-extra-derives="strum::EnumString" --enum-extra-attributes="serde(rename_all = \"snake_case\")"
