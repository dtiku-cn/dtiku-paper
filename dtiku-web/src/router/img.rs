use crate::{plugins::OpenListConfig, rpc};
use anyhow::Context;
use axum_extra::extract::Multipart;
use chrono::{Datelike, Local};
use dtiku_paper::model::Assets;
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::json;
use spring::tracing;
use spring_opendal::Op;
use spring_sea_orm::DbConn;
use spring_web::{
    axum::{
        response::{IntoResponse, Redirect},
        Json,
    },
    error::{KnownWebError, Result},
    extractor::{Component, Config, Path},
    get, post,
};
use uuid::Uuid;

#[post("/upload")]
async fn upload(
    Component(dav): Component<Op>,
    Config(config): Config<OpenListConfig>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    while let Some(field) = multipart
        .next_field()
        .await
        .context("get multipart field failed")?
    {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "file" {
            let (dir_path, file_path) = dir_and_file_path();
            dav.create_dir(&format!("{dir_path}/"))
                .await
                .with_context(|| format!("mkdir for {dir_path} failed"))?;
            let resp = dav
                .write(&file_path, data)
                .await
                .with_context(|| format!("upload to {file_path} failed"))?;
            tracing::info!("upload ==> {resp:?}");
            let url = rpc::alist::get_file_path(&file_path, &config)
                .await
                .with_context(|| format!("get_file_info({file_path}) failed"))?;
            return Ok(url);
        }
    }
    Err(KnownWebError::bad_request("上传请求不正确").into())
}

fn dir_and_file_path() -> (String, String) {
    let now = Local::now();
    let year = now.year();
    let month = now.month();
    let day = now.day();
    let dir_path = format!("pan.wo/artalk/{}/{:02}/{:02}", year, month, day);
    let uid = Uuid::new_v4();
    let file_path = format!("{dir_path}/{uid}");
    (dir_path, file_path)
}

#[derive(Debug, Deserialize)]
struct UploadUrlReq {
    pub url: String,
}

#[post("/upload-by-link")]
async fn upload_by_link(
    Component(dav): Component<Op>,
    Config(config): Config<OpenListConfig>,
    Json(url_req): Json<UploadUrlReq>,
) -> Result<impl IntoResponse> {
    let original_url = url_req.url;
    let resp = reqwest::get(&original_url)
        .await
        .with_context(|| format!("图片请求失败:{original_url}"))?;
    let data = resp
        .bytes()
        .await
        .with_context(|| format!("图片下载失败:{original_url}"))?;
    let (dir_path, file_path) = dir_and_file_path();
    dav.create_dir(&format!("{dir_path}/"))
        .await
        .with_context(|| format!("mkdir for {dir_path} failed"))?;
    let resp = dav
        .write(&file_path, data)
        .await
        .with_context(|| format!("upload to {file_path} failed"))?;
    tracing::info!("upload ==> {resp:?}");
    let url = rpc::alist::get_file_path(&file_path, &config)
        .await
        .with_context(|| format!("get_file_info({file_path}) failed"))?;
    Ok(Json(json!({
        "msg": "上传成功",
        "code": 0,
        "data": {
            "originalURL": original_url,
            "url": url
        }
    })))
}

#[get("/assets/{year}/{month}/{day}/{id}")]
async fn get_img(
    Component(db): Component<DbConn>,
    Config(config): Config<OpenListConfig>,
    Path((year, month, day, id)): Path<(i32, i32, i32, i32)>,
) -> Result<impl IntoResponse> {
    let file_path = format!("/assets/{year}/{month}/{day}/{id}");
    if config.use_origin {
        let assets = Assets::find_by_id(id)
            .one(&db)
            .await
            .with_context(|| format!("find_assets_by_id({id}) failed"))?;
        if let Some(a) = assets {
            return Ok(Redirect::permanent(&a.compute_src_url()));
        }
    };

    let alist_url = rpc::alist::get_file_path(&file_path, &config)
        .await
        .with_context(|| format!("get_file_info({file_path}) failed"))?;
    return Ok(Redirect::permanent(&alist_url));
}
