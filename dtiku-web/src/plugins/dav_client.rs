use super::WebDAVClientConfig;
use derive_more::derive::{Deref, DerefMut};
use reqwest_dav::{Auth, Client, ClientBuilder};
use spring::{
    app::AppBuilder,
    async_trait,
    config::ConfigRegistry,
    plugin::{MutableComponentRegistry, Plugin},
};

pub struct WebDAVClientPlugin;

#[async_trait]
impl Plugin for WebDAVClientPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let dav_config = app
            .get_config::<WebDAVClientConfig>()
            .expect("load huggingface config failed");

        let client = ClientBuilder::new()
            .set_host(dav_config.host)
            .set_auth(Auth::Basic(dav_config.username, dav_config.password))
            .build()
            .expect("build webdav client failed");

        app.add_component(WebDAVClient(client));
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct WebDAVClient(Client);
