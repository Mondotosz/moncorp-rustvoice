#[derive(Clone, sqlx::FromRow, Debug)]
pub struct PrimaryChannel {
    pub id: i64,
}

#[derive(Clone, sqlx::FromRow, Debug)]
pub struct TemporaryChannel {
    pub id: i64,
}
