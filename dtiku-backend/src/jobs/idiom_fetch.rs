use reqwest_scraper::FromCssSelector;

#[derive(Debug, FromCssSelector)]
struct Idiom {
    #[selector(path = "#main div.words-details h4>span", default="<uname>", text)]
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
