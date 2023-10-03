use std::env;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::time::{sleep, Duration};

use unisub::{run_migrations, Error, PubSub};

// DATABASE_URL="postgres://postgres:postgres@localhost:5432" cargo run --example bincoded
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize the database connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Initialize the PubSub system
    let mut pubsub = PubSub::new(pool.clone())
        .await
        .expect("Failed to initialize PubSub");

    // Create a topic (let's call it "my_topic")
    pubsub.create_topic("my_topic").await.ok();

    // Create some data to publish
    let data = MyData {
        field1: "Hello".to_string(),
        field2: 42,
    };

    // Serialize the data using bincode
    let encoded = bincode::serialize(&data).expect("Failed to serialize data");

    // Publish the data to "my_topic"
    pubsub
        .push("my_topic", &encoded)
        .await
        .expect("Failed to publish data");

    // select with a timeout so this example doesn't run forever
    tokio::select! {
        _ = pubsub
        .subscribe("my_topic", |message: Vec<u8>| {
            async move {
                // Deserialize the received data using bincode
                let decoded: MyData =
                    bincode::deserialize(&message).expect("Failed to deserialize data");

                // Process the data (Here, we're just printing it out)
                println!("Received and decoded data: {:?}", decoded);

                Ok(())
            }
        }) => {},
        _ = sleep(Duration::from_secs(1)) => {},
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct MyData {
    field1: String,
    field2: i32,
}
