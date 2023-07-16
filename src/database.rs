use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

async fn connect_to_database() -> Result<PgPool, sqlx::Error> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&*database_url)
        .await?;
    Ok(pool)
}
