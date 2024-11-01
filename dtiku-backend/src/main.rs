use spring::App;
use spring_sea_orm::SeaOrmPlugin;
use spring_stream::StreamPlugin;
use spring_web::WebPlugin;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .add_plugin(StreamPlugin)
        .run()
        .await
}
