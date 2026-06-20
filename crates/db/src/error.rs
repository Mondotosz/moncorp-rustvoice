use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error("I/O error setting up database: {0}")]
    Io(#[from] std::io::Error),
}
