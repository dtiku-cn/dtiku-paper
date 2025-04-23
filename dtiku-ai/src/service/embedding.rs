pub mod proto {
    tonic::include_proto!("embedding");
}

use fastembed::{ImageEmbedding, TextEmbedding};
use proto::embedding_service_server::EmbeddingService;
use proto::{BatchTextReq, Embedding, TensorResp, TextReq};
use tonic::{Request, Response, Status};

pub struct EmbeddingServiceImpl {
    pub text_embedding: TextEmbedding,
    pub image_embedding: ImageEmbedding,
}

#[tonic::async_trait]
impl EmbeddingService for EmbeddingServiceImpl {
    async fn text_embedding(
        &self,
        request: Request<TextReq>,
    ) -> Result<Response<Embedding>, Status> {
        todo!()
    }
    async fn batch_text_embedding(
        &self,
        request: Request<BatchTextReq>,
    ) -> Result<Response<TensorResp>, Status> {
        todo!()
    }
}
