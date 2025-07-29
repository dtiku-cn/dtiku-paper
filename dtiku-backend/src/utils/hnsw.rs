use hnsw_rs::prelude::*;
use ouroboros::self_referencing;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LabeledSentence {
    pub id: usize,
    pub label: String, // "question" or "solution"
    pub outer_id: i32,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[self_referencing]
pub struct HNSWIndex {
    sentences_map: HashMap<usize, LabeledSentence>,
    #[borrows(sentences_map)]
    #[not_covariant]
    hnsw: Hnsw<'this, f32, DistCosine>,
}

impl HNSWIndex {
    pub fn build(sentences: &[LabeledSentence]) -> Self {
        let mut sentences_map = HashMap::with_capacity(sentences.len());
        for sentence in sentences {
            sentences_map.insert(sentence.id, sentence.clone());
        }
        HNSWIndexBuilder {
            sentences_map,
            hnsw_builder: |sm| {
                let hnsw =
                    Hnsw::<f32, DistCosine>::new(48, sm.len() + 1, 16, 800, DistCosine::default());
                for sentence in sm.values() {
                    hnsw.insert((&sentence.embedding, sentence.id));
                }
                hnsw
            },
        }
        .build()
    }

    /// 查询最近的 top-k 个结果
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(&LabeledSentence, f32)> {
        self.with(|fields| {
            let hnsw = &fields.hnsw;
            let sentences_map = &fields.sentences_map;

            hnsw.search(query, k, k * 4)
                .iter()
                .filter_map(|point| sentences_map.get(&point.d_id).map(|m| (m, point.distance)))
                .collect()
        })
    }
}
