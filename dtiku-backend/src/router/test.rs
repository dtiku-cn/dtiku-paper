use crate::plugins::embedding::Embedding;
use crate::utils::regex as regex_util;
use crate::{
    service::nlp::NLPService,
    views::test::{TextCompare, WebLabelReq},
};
use anyhow::Context as _;
use gaoya::minhash::{MinHasher, MinHasher64V1};
use gaoya::simhash::SimSipHasher128;
use reqwest_scraper::ScraperResponse;
use serde_json::json;
use spring_web::{
    axum::{response::IntoResponse, Json},
    error::{KnownWebError, Result},
    extractor::{Component, Path, Query},
    get, post,
};

#[post("/api/text_similarity")]
async fn test_text_similarity(Json(q): Json<TextCompare>) -> Result<impl IntoResponse> {
    let TextCompare { source, target } = q;
    let bag = textdistance::str::bag(&source, &target);
    let cosine = textdistance::str::cosine(&source, &target);
    let damerau_levenshtein = textdistance::str::damerau_levenshtein(&source, &target);
    let damerau_levenshtein_restricted =
        textdistance::str::damerau_levenshtein_restricted(&source, &target);
    let entropy_ncd = textdistance::str::entropy_ncd(&source, &target);
    let hamming = textdistance::str::hamming(&source, &target);
    let jaccard = textdistance::str::jaccard(&source, &target);
    let jaro = textdistance::str::jaro(&source, &target);
    let jaro_winkler = textdistance::str::jaro_winkler(&source, &target);
    let lcsseq = textdistance::str::lcsseq(&source, &target);
    let lcsstr = textdistance::str::lcsstr(&source, &target);
    let levenshtein = textdistance::str::levenshtein(&source, &target);
    let lig3 = textdistance::str::lig3(&source, &target);
    let mlipns = textdistance::str::mlipns(&source, &target);
    let overlap = textdistance::str::overlap(&source, &target);
    let prefix = textdistance::str::prefix(&source, &target);
    let ratcliff_obershelp = textdistance::str::ratcliff_obershelp(&source, &target);
    let roberts = textdistance::str::roberts(&source, &target);
    let sift4_common = textdistance::str::sift4_common(&source, &target);
    let sift4_simple = textdistance::str::sift4_simple(&source, &target);
    let smith_waterman = textdistance::str::smith_waterman(&source, &target);
    let sorensen_dice = textdistance::str::sorensen_dice(&source, &target);
    let suffix = textdistance::str::suffix(&source, &target);
    let tversky = textdistance::str::tversky(&source, &target);
    let yujian_bo = textdistance::str::yujian_bo(&source, &target);
    let min_hash = MinHasher64V1::new(200);
    let source_min_hash = min_hash.create_signature(source.chars());
    let target_min_hash = min_hash.create_signature(target.chars());
    let min_hash_similarity = min_hash.compute_similarity(source.chars(), target.chars());
    // let sim_hash = SimSipHasher128::new(200, 200);
    // sim_hash.hash();
    Ok(Json(json!({
        "source_min_hash": source_min_hash,
        "target_min_hash": target_min_hash,
        "min_hash_similarity":min_hash_similarity,
        "bag":bag,
        "cosine":cosine,
        "damerau_levenshtein":damerau_levenshtein,
        "damerau_levenshtein_restricted":damerau_levenshtein_restricted,
        "entropy_ncd":entropy_ncd,
        "hamming":hamming,
        "jaccard":jaccard,
        "jaro":jaro,
        "jaro_winkler":jaro_winkler,
        "lcsseq":lcsseq,
        "lcsstr":lcsstr,
        "levenshtein":levenshtein,
        "lig3":lig3,
        "mlipns":mlipns,
        "overlap":overlap,
        "prefix":prefix,
        "ratcliff_obershelp":ratcliff_obershelp,
        "roberts":roberts,
        "sift4_common":sift4_common,
        "sift4_simple":sift4_simple,
        "smith_waterman":smith_waterman,
        "sorensen_dice":sorensen_dice,
        "suffix":suffix,
        "tversky":tversky,
        "yujian_bo":yujian_bo
    })))
}

#[get("/api/web_text_extract")]
async fn test_web_text_extract(Query(req): Query<WebLabelReq>) -> Result<impl IntoResponse> {
    let url = url::Url::parse(&req.url).with_context(|| format!("parse url failed:{}", req.url))?;
    let html = reqwest::Client::builder().user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0")
        .build()
        .unwrap()
        .get(url.clone())
        .send()
        .await
        .context("reqwest::get failed")?
        .html()
        .await
        .context("get response text failed")?;
    let mut html_reader = std::io::Cursor::new(html.clone());
    let readability_page = readability::extractor::extract(&mut html_reader, &url)
        .context("readability::extractor::extract failed")?;

    let mut readability = dom_smoothie::Readability::new(html.clone(), Some(&req.url), None)
        .context("create dom_smoothie::Readability failed")?;
    let dom_smoothie_article: dom_smoothie::Article =
        readability.parse().context("parse html failed")?;
    //let extractor = extractous::Extractor::new();

    Ok(Json(json!({
        "raw_html": html,
        "readability_page": {
            "title": readability_page.title,
            "content": readability_page.content,
            "text": readability_page.text
        },
        "dom_smoothie_article": dom_smoothie_article
    })))
}

#[get("/api/web_text_label/{question_id}")]
async fn test_web_text_label(
    Component(nlp): Component<NLPService>,
    Component(embedding): Component<Embedding>,
    Path(question_id): Path<i32>,
    Query(req): Query<WebLabelReq>,
) -> Result<impl IntoResponse> {
    let hnsw = nlp
        .build_hnsw_index_for_question(question_id)
        .await?
        .ok_or_else(|| KnownWebError::not_found("问题不存在"))?;
    let url = url::Url::parse(&req.url).with_context(|| format!("parse url failed:{}", req.url))?;
    let html = reqwest::Client::builder().user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0")
        .build()
        .unwrap()
        .get(url.clone())
        .send()
        .await
        .context("reqwest::get failed")?
        .html()
        .await
        .context("get response text failed")?;
    let mut html_reader = std::io::Cursor::new(html.clone());

    let readability_page = readability::extractor::extract(&mut html_reader, &url)
        .context("readability::extractor::extract failed")?;
    let text = &readability_page.text;

    let mut label_sentences = vec![];
    for sentence in regex_util::split_sentences(&text) {
        let embedding = embedding.text_embedding(sentence).await?;
        let s = hnsw.search(&embedding, 1);
        let ls = if s.is_empty() {
            json!({
                "sentence":sentence
            })
        } else {
            let label = s[0].label.clone();
            json!({
                "sentence": sentence,
                "label": label
            })
        };
        label_sentences.push(ls);
    }
    Ok(Json(json!({
        "text":text,
        "labeled_text":label_sentences
    })))
}
