use hnsw_rs::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LabeledSentence {
    pub id: usize,
    pub label: String, // "question" or "solution"
    pub text: String,
    pub embedding: Vec<f32>,
}

pub struct HNSWIndex<'e> {
    sentences_map: HashMap<usize, LabeledSentence>,
    hnsw: Hnsw<'e, f32, DistCosine>,
}

pub fn build_hnsw_index(sentences: &[LabeledSentence]) -> HNSWIndex {
    let hnsw = Hnsw::<f32, DistCosine>::new(16, sentences.len(), 0, 160, DistCosine::default());
    let mut sentences_map = HashMap::with_capacity(sentences.len());
    for sentence in sentences {
        sentences_map.insert(sentence.id, sentence.clone());
        hnsw.insert((&sentence.embedding, sentence.id));
    }
    HNSWIndex {
        sentences_map,
        hnsw,
    }
}
