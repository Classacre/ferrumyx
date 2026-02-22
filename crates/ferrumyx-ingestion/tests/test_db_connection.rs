//! Test database connection directly.
//!
//! Run with: cargo test --package ferrumyx-ingestion --test test_db_connection -- --ignored --nocapture

use sqlx::postgres::PgPoolOptions;

#[tokio::test]
#[ignore]
async fn test_db_connection() {
    let database_url = "postgres://ferrumyx:ferrumyx@localhost:5432/ferrumyx";
    
    println!("Connecting to: {}", database_url);
    
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await;
    
    match pool {
        Ok(p) => {
            println!("Connected successfully!");
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM papers")
                .fetch_one(&p)
                .await
                .unwrap();
            println!("Papers count: {}", row.0);
        }
        Err(e) => {
            println!("Connection failed: {:?}", e);
            panic!("Could not connect");
        }
    }
}
