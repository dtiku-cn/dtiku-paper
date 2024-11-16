use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("lock error: {0}")]
    OptimisticLockErr(String),

    #[error(transparent)]
    DbErr(#[from] sea_orm::error::DbErr),
}
