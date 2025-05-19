pub mod proto {
    tonic::include_proto!("embedding");
}

use super::AnyhowToStatus;
use crate::plugins::fastembed::TxtEmbedding;
use proto::embedding_service_server::{EmbeddingService, EmbeddingServiceServer};
use proto::{BatchTextReq, Embedding, TensorResp, TextReq};
use spring::plugin::service::Service;
use tonic::{Code, Request, Response, Status};

#[derive(Clone, Service)]
#[service(grpc = "EmbeddingServiceServer")]
pub struct EmbeddingServiceImpl {
    #[inject(component)]
    pub text_embedding: TxtEmbedding,
    // #[inject(component)]
    // pub image_embedding: ImgEmbedding,
}

#[tonic::async_trait]
impl EmbeddingService for EmbeddingServiceImpl {
    async fn text_embedding(
        &self,
        request: Request<TextReq>,
    ) -> Result<Response<Embedding>, Status> {
        let text = request.into_inner().text;
        let mut embedding = self
            .text_embedding
            .embed(vec![text], None)
            .map_err(|e| e.to_status(Code::Internal))?;
        let embedding = embedding.remove(0);
        Ok(Response::new(Embedding {
            embedding: embedding.into(),
        }))
    }

    async fn batch_text_embedding(
        &self,
        request: Request<BatchTextReq>,
    ) -> Result<Response<TensorResp>, Status> {
        let BatchTextReq { texts, batch_size } = request.into_inner();
        let embeddings = self
            .text_embedding
            .embed(texts, Some(batch_size as usize))
            .map_err(|e| e.to_status(Code::Internal))?;
        Ok(Response::new(TensorResp {
            embeddings: embeddings
                .into_iter()
                .map(|embedding| Embedding { embedding })
                .collect(),
        }))
    }
}
