use sqlx::postgres::PgPoolOptions;

pub async fn setup_db(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
