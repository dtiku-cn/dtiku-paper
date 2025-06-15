use reqwest;
use reqwest_scraper::{FromCssSelector, ScraperResponse};

#[derive(Debug, FromCssSelector)]
pub struct Idiom {
    #[selector(path = "#main div.words-details h4>span", default = "<uname>", text)]
    idiom: String,

    #[selector(path = "#shiyiDiv", inner_html)]
    shiyi: Option<String>,

    #[selector(path = "#shiyidetailDiv", inner_html)]
    shiyidetail: Option<String>,

    #[selector(path = "#liju ul.item-list", html)]
    liju: Option<String>,

    #[selector(path = "#jyc ul.words-list>li a.text-default", text)]
    jyc: Vec<String>,

    #[selector(path = "#fyc ul.words-list>li a.text-default", text)]
    fyc: Vec<String>,
}

impl Idiom {
    pub async fn fetch_explain(idiom: &str) -> anyhow::Result<Self> {
        let html = reqwest::get(format!("https://hanyu.sogou.com/result?query={idiom}"))
            .await?
            .css_selector()
            .await?;

        Ok(Self::from_html(html)?)
    }
}
