use crate::{plugins::OpenListConfig, rpc};
use anyhow::Context;
use axum_extra::extract::Multipart;
use chrono::{Datelike, Local};
use spring::tracing;
use spring_opendal::Op;
use spring_web::{
    axum::response::IntoResponse,
    error::{KnownWebError, Result},
    extractor::{Component, Config},
    post,
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
            let now = Local::now();
            let year = now.year();
            let month = now.month();
            let day = now.day();
            let dir_path = format!("pan.wo/artalk/{}/{:02}/{:02}", year, month, day);
            let uid = Uuid::new_v4();
            let file_path = format!("{dir_path}/{uid}");
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
