use crate::model::paper_question;
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;
use sea_orm::{prelude::Expr, sea_query::IntoCondition, ColumnTrait};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use validator::Validate;

static KEY_POINT_PATH: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+(.\d+)?").unwrap());

#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
pub struct PaperQuestionQuery {
    #[serde(default)]
    pub paper_type: i16,
    #[validate(length(min = 1, max = 20))]
    #[serde(default, rename = "pid")]
    pub paper_ids: Vec<i32>,
    #[validate(regex(path = *KEY_POINT_PATH))]
    #[serde(default, rename = "kp_path")]
    pub keypoint_path: String,
    #[serde(default, rename = "correct_ratio")]
    pub correct_ratio: CorrectRatio,
}

impl IntoCondition for PaperQuestionQuery {
    fn into_condition(self) -> sea_orm::sea_query::Condition {
        let mut cond = sea_orm::sea_query::Condition::all();
        if self.paper_type != 0 {
            cond = cond.add(paper_question::Column::PaperType.eq(self.paper_type));
        }
        if !self.paper_ids.is_empty() {
            cond = cond.add(paper_question::Column::PaperId.is_in(self.paper_ids));
        }
        if !self.keypoint_path.is_empty() {
            cond = cond.add(Expr::cust(format!(
                "keypoint_path <@ '{}'::ltree",
                self.keypoint_path
            )));
        }
        if self.correct_ratio.0 != 0.0 || self.correct_ratio.1 != 100.0 {
            let ratio = self.correct_ratio;
            cond = cond.add(paper_question::Column::CorrectRatio.between(ratio.0, ratio.1));
        }
        cond
    }
}

#[derive(Debug, Clone, Display)]
#[display("{_0},{_1}")]
pub struct CorrectRatio(pub f32, pub f32);

impl Default for CorrectRatio {
    fn default() -> Self {
        CorrectRatio(0.0, 100.0)
    }
}

impl Serialize for CorrectRatio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{},{}", self.0, self.1);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for CorrectRatio {
    fn deserialize<D>(deserializer: D) -> Result<CorrectRatio, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom("expected format \"x,y\""));
        }
        let x = parts[0].parse::<f32>().map_err(serde::de::Error::custom)?;
        let y = parts[1].parse::<f32>().map_err(serde::de::Error::custom)?;
        Ok(CorrectRatio(x, y))
    }
}
