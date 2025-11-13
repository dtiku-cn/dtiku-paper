use crate::{
    domain::question::FullQuestion,
    model::paper::{self, PaperChapter, PaperExtra},
};
use serde::Deserialize;
use std::collections::HashMap;
use strum::{AsRefStr, Display, EnumIter, EnumMessage, EnumString};

#[derive(
    Default, Debug, Clone, Copy, Deserialize, Display, EnumIter, AsRefStr, EnumString, EnumMessage,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaperMode {
    #[strum(message = "练习模式")]
    Exercise,
    #[default]
    #[strum(message = "展示答案")]
    ShowAnswer,
}

impl PaperMode {
    pub fn text(&self) -> &'static str {
        self.get_message().unwrap_or_default()
    }
}

pub struct FullPaper {
    pub mode: PaperMode,
    pub p: crate::model::paper::Model,
    pub qs: Vec<crate::model::question::Question>,
    pub ms: Vec<crate::model::material::Material>,
    pub ss: Vec<crate::model::solution::Model>,
    pub qid_mid_map: HashMap<i32, Vec<i32>>,
}

#[derive(Debug, Clone)]
pub struct ChapterReport {
    pub chapter: PaperChapter,
    pub correct: u64,
    pub error: u64,
    pub time: u64,
}

impl ChapterReport {
    pub fn correct_ratio(&self) -> String {
        if self.chapter.count == 0 {
            "0%".to_string()
        } else {
            format!(
                "{:.1}%",
                100.0 * (self.correct as f64) / (self.chapter.count as f64)
            )
        }
    }
}

impl FullPaper {
    pub(crate) fn new(
        mode: PaperMode,
        p: crate::model::paper::Model,
        qs: Vec<crate::model::question::Question>,
        ms: Vec<crate::model::material::Material>,
        ss: Vec<crate::model::solution::Model>,
        qid_map: HashMap<i32, Vec<i32>>,
    ) -> Self {
        Self {
            mode,
            p,
            qs,
            ms,
            ss,
            qid_mid_map: qid_map,
        }
    }
}

pub fn compute_report(
    paper: &paper::Model,
    questions: &Vec<FullQuestion>,
    user_answers: &HashMap<i32, String>,
    id_time_map: &HashMap<i32, u64>,
) -> Vec<ChapterReport> {
    let chapters = if let PaperExtra::Chapters(chapters) = &paper.extra {
        chapters
    } else {
        panic!("申论暂时不支持批改")
    };
    // compute_paper_chapter_range 返回 Vec<(start..=end, chapter)>
    let number_range = chapters.compute_paper_chapter_range();

    let mut correct: HashMap<PaperChapter, u64> = HashMap::new();
    let mut error: HashMap<PaperChapter, u64> = HashMap::new();
    let mut time: HashMap<PaperChapter, u64> = HashMap::new();

    let mut total_correct = 0;
    let mut total_error = 0;
    let mut total_time = 0;

    for q in questions {
        let answer_in_db = q.get_raw_answer();
        let user_answer = user_answers.get(&q.id).map(|s| s.as_str());

        let chapter = number_range
            .iter()
            .find(|(range, _)| range.contains(&q.num))
            .map(|(_, ch)| ch.clone())
            .expect("Question number not in any chapter");

        if let (Some(db), Some(user)) = (answer_in_db, user_answer) {
            if db.eq_ignore_ascii_case(user) {
                *correct.entry(chapter.clone()).or_default() += 1;
                total_correct += 1;
            } else {
                *error.entry(chapter.clone()).or_default() += 1;
                total_error += 1;
            }
        } else {
            *error.entry(chapter.clone()).or_default() += 1;
            total_error += 1;
        }

        let t = *id_time_map.get(&q.id).unwrap_or(&0);
        *time.entry(chapter.clone()).or_default() += t;
        total_time += t;
    }

    let mut stats: Vec<ChapterReport> = chapters
        .chapters
        .iter()
        .map(|c| ChapterReport {
            chapter: c.clone(),
            correct: *correct.get(c).unwrap_or(&0),
            error: *error.get(c).unwrap_or(&0),
            time: *time.get(c).unwrap_or(&0),
        })
        .collect();

    stats.push(ChapterReport {
        chapter: PaperChapter {
            name: "合计".to_string(),
            desc: "".to_string(),
            count: 0,
        },
        correct: total_correct,
        error: total_error,
        time: total_time,
    });

    stats
}
