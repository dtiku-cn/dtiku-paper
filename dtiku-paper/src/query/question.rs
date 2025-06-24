use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PaperQuestionQuery {
    #[serde(default)]
    pub paper_type: i16,
    #[serde(default, rename = "pid")]
    pub paper_ids: Vec<i32>,
    #[serde(default, rename = "kp_path")]
    pub keypoint_path: String,
    #[serde(default, rename = "correct_ratio")]
    pub correct_ratio: CorrectRatio,
}

#[derive(Debug, Clone)]
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
