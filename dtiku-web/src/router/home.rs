use crate::data::home::HomeTemplate;
use spring_web::get;

#[get("/")]
async fn home() -> HomeTemplate {
    println!("index");
    HomeTemplate {}
}
