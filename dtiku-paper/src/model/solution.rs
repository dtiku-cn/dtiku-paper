pub use super::_entities::solution::*;
use crate::{
    model::{assets, SrcType},
    util::html,
};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait as _, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    EntityTrait, FromJsonQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_with::{formats::CommaSeparator, serde_as, StringWithSeparator};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(tag = "type")]
pub enum SolutionExtra {
    // 单选题
    #[serde(rename = "sc")]
    SingleChoice(SingleChoice),
    // 多选题
    #[serde(rename = "mc")]
    MultiChoice(MultiChoice),
    // 不定项选择题
    #[serde(rename = "ic")]
    IndefiniteChoice(MultiChoice),
    // 完形填空选择题
    #[serde(rename = "bc")]
    BlankChoice(SingleChoice),
    // 填空题
    #[serde(rename = "fb")]
    FillBlank(FillBlank),
    // 填空题
    #[serde(rename = "ba")]
    BlankAnswer(BlankAnswer),
    // 是非判断题
    #[serde(rename = "tf")]
    TrueFalse(TrueFalseChoice),
    // 封闭式解答题
    #[serde(rename = "ce")]
    ClosedEndedQA(AnswerAnalysis),
    // 开放式解答题
    #[serde(rename = "oe")]
    OpenEndedQA(StepByStepAnswer),
    // 其他问答题
    #[serde(rename = "o")]
    OtherQA(OtherAnswer),
}

impl SolutionExtra {
    pub fn is_answer(&self, index0: usize) -> bool {
        let answer_index = index0 as u8;
        match self {
            Self::SingleChoice(SingleChoice { answer, .. })
            | Self::BlankChoice(SingleChoice { answer, .. }) => *answer == answer_index,
            Self::MultiChoice(MultiChoice { answer, .. })
            | Self::IndefiniteChoice(MultiChoice { answer, .. }) => answer.contains(&answer_index),
            Self::TrueFalse(TrueFalseChoice { answer, .. }) => *answer && answer_index == 0,
            _ => false,
        }
    }

    pub fn get_answer(&self) -> Option<String> {
        match self {
            Self::SingleChoice(SingleChoice { answer, .. })
            | Self::BlankChoice(SingleChoice { answer, .. }) => Some(Self::convert_answer(*answer)),
            Self::MultiChoice(MultiChoice { answer, .. })
            | Self::IndefiniteChoice(MultiChoice { answer, .. }) => {
                Some(answer.iter().map(|a| Self::convert_answer(*a)).join(", "))
            }
            Self::TrueFalse(TrueFalseChoice { answer, .. }) => {
                if *answer {
                    Some("T".to_string())
                } else {
                    Some("F".to_string())
                }
            }
            Self::BlankAnswer(BlankAnswer { answer, .. }) => Some(answer.clone()),
            Self::FillBlank(FillBlank { blanks, .. }) => Some(blanks.join(" ")),
            Self::OtherQA(OtherAnswer { answer, .. }) => answer.clone(),
            _ => None,
        }
    }

    fn convert_answer(answer: u8) -> String {
        let c = (b'A' + answer) as char;
        c.to_string()
    }

    pub fn get_html(&self) -> String {
        match self {
            Self::SingleChoice(SingleChoice { analysis, .. })
            | Self::BlankChoice(SingleChoice { analysis, .. })
            | Self::MultiChoice(MultiChoice { analysis, .. })
            | Self::IndefiniteChoice(MultiChoice { analysis, .. })
            | Self::TrueFalse(TrueFalseChoice { analysis, .. })
            | Self::FillBlank(FillBlank { analysis, .. })
            | Self::BlankAnswer(BlankAnswer { analysis, .. }) => analysis.to_string(),
            Self::ClosedEndedQA(AnswerAnalysis { answer, .. }) => answer.to_string(),
            Self::OpenEndedQA(StepByStepAnswer { solution, analysis }) => {
                if let Some(sol) = solution {
                    if !sol.is_empty() {
                        return sol.to_string();
                    }
                }
                analysis
                    .iter()
                    .filter(|s| ["demonstrate", "reference", "sfdt"].contains(&s.label.as_str()))
                    .map(|s| s.content.as_str())
                    .join("。")
            }
            Self::OtherQA(OtherAnswer {
                answer,
                solution,
                analysis,
            }) => {
                if let Some(ans) = answer {
                    ans.to_string()
                } else if let Some(sol) = solution {
                    sol.to_string()
                } else {
                    analysis
                        .iter()
                        .filter(|s| {
                            ["demonstrate", "reference", "sfdt"].contains(&s.label.as_str())
                        })
                        .map(|s| s.content.as_str())
                        .join("。")
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct SingleChoice {
    pub answer: u8,
    pub analysis: String,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct MultiChoice {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, u8>")]
    pub answer: Vec<u8>,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct TrueFalseChoice {
    pub answer: bool,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct FillBlank {
    pub blanks: Vec<String>,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct BlankAnswer {
    pub answer: String,
    pub analysis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StepByStepAnswer {
    pub solution: Option<String>,
    pub analysis: Vec<StepAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct OtherAnswer {
    pub answer: Option<String>,
    pub solution: Option<String>,
    pub analysis: Vec<StepAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StepAnalysis {
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct AnswerAnalysis {
    pub answer: String,
    pub analysis: String,
}

impl Entity {
    pub async fn find_by_qid<C>(db: &C, question_id: i32) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
    {
        Entity::find()
            .filter(Column::QuestionId.eq(question_id))
            .all(db)
            .await
            .with_context(|| format!("find_by_question_id({question_id}) failed"))
    }

    pub async fn find_by_question_ids<C, IDS>(
        db: &C,
        question_ids: IDS,
    ) -> anyhow::Result<Vec<Model>>
    where
        C: ConnectionTrait,
        IDS: IntoIterator<Item = i32>,
    {
        Entity::find()
            .filter(Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
            .with_context(|| format!("find_by_question_ids failed"))
    }
}

impl ActiveModel {
    pub async fn insert_on_conflict<C: ConnectionTrait>(self, db: &C) -> anyhow::Result<Model> {
        let model = Entity::insert(self)
            .on_conflict(
                OnConflict::columns([Column::QuestionId, Column::FromTy])
                    .update_columns([Column::Extra])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await
            .context("insert solution failed")?;

        let replaced_extra = match model.extra.clone() {
            SolutionExtra::SingleChoice(mut sc) => {
                sc.analysis = model
                    .replace_img_src(db, &sc.analysis)
                    .await
                    .expect("replace SingleChoice::analysis failed");
                SolutionExtra::SingleChoice(sc)
            }
            SolutionExtra::MultiChoice(mut mc) => {
                mc.analysis = model
                    .replace_img_src(db, &mc.analysis)
                    .await
                    .expect("replace MultiChoice::analysis failed");
                SolutionExtra::MultiChoice(mc)
            }
            SolutionExtra::IndefiniteChoice(mut mc) => {
                mc.analysis = model
                    .replace_img_src(db, &mc.analysis)
                    .await
                    .expect("replace IndefiniteChoice::analysis failed");
                SolutionExtra::IndefiniteChoice(mc)
            }
            SolutionExtra::BlankChoice(mut sc) => {
                sc.analysis = model
                    .replace_img_src(db, &sc.analysis)
                    .await
                    .expect("replace BlankChoice::analysis failed");
                SolutionExtra::BlankChoice(sc)
            }
            SolutionExtra::FillBlank(mut fb) => {
                let mut new_blanks = vec![];
                for b in fb.blanks {
                    new_blanks.push(
                        model
                            .replace_img_src(db, &b)
                            .await
                            .expect("replace FillBlank::blanks failed"),
                    )
                }
                fb.blanks = new_blanks;
                fb.analysis = model
                    .replace_img_src(db, &fb.analysis)
                    .await
                    .expect("replace FillBlank::analysis failed");
                SolutionExtra::FillBlank(fb)
            }
            SolutionExtra::BlankAnswer(mut ba) => {
                ba.answer = model
                    .replace_img_src(db, &ba.answer)
                    .await
                    .expect("replace BlankAnswer::answer failed");
                ba.analysis = model
                    .replace_img_src(db, &ba.analysis)
                    .await
                    .expect("replace BlankAnswer::analysis failed");
                SolutionExtra::BlankAnswer(ba)
            }
            SolutionExtra::TrueFalse(mut sc) => {
                sc.analysis = model
                    .replace_img_src(db, &sc.analysis)
                    .await
                    .expect("replace TrueFalse::analysis failed");
                SolutionExtra::TrueFalse(sc)
            }
            SolutionExtra::ClosedEndedQA(mut aa) => {
                aa.answer = model
                    .replace_img_src(db, &aa.answer)
                    .await
                    .expect("replace ClosedEndedQA::answer failed");
                aa.analysis = model
                    .replace_img_src(db, &aa.analysis)
                    .await
                    .expect("replace ClosedEndedQA::analysis failed");
                SolutionExtra::ClosedEndedQA(aa)
            }
            SolutionExtra::OpenEndedQA(mut ssa) => {
                if let Some(s) = ssa.solution {
                    ssa.solution = Some(
                        model
                            .replace_img_src(db, &s)
                            .await
                            .expect("replace OpenEndedQA::solution failed"),
                    );
                }
                let mut new_analysis = vec![];
                for StepAnalysis { label, content } in ssa.analysis {
                    new_analysis.push(StepAnalysis {
                        label,
                        content: model
                            .replace_img_src(db, &content)
                            .await
                            .expect("replace OpenEndedQA::analysis failed"),
                    });
                }
                ssa.analysis = new_analysis;
                SolutionExtra::OpenEndedQA(ssa)
            }
            SolutionExtra::OtherQA(mut oqa) => {
                if let Some(answer) = oqa.answer {
                    oqa.answer = Some(
                        model
                            .replace_img_src(db, &answer)
                            .await
                            .expect("replace OpenEndedQA::answer failed"),
                    );
                }
                if let Some(s) = oqa.solution {
                    oqa.solution = Some(
                        model
                            .replace_img_src(db, &s)
                            .await
                            .expect("replace OpenEndedQA::solution failed"),
                    );
                }
                let mut new_analysis = vec![];
                for StepAnalysis { label, content } in oqa.analysis {
                    new_analysis.push(StepAnalysis {
                        label,
                        content: model
                            .replace_img_src(db, &content)
                            .await
                            .expect("replace OpenEndedQA::analysis failed"),
                    });
                }
                oqa.analysis = new_analysis;
                SolutionExtra::OtherQA(oqa)
            }
        };
        ActiveModel {
            id: Set(model.id),
            extra: Set(replaced_extra),
            ..Default::default()
        }
        .update(db)
        .await
        .context("update solution extra failed")
    }
}

impl Model {
    async fn replace_img_src<C: ConnectionTrait>(
        &self,
        db: &C,
        content: &str,
    ) -> anyhow::Result<String> {
        html::async_replace_img_src(content, |img_url| {
            let img_url = img_url.to_string();
            Box::pin(async move {
                let assets = assets::SourceAssets {
                    src_type: SrcType::Solution,
                    src_id: self.id,
                    src_url: img_url,
                }
                .insert_on_conflict(db)
                .await?;
                Ok(assets.compute_storage_url())
            })
        })
        .await
    }
}
