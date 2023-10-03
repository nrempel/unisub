// To run this integration test, you need to have a Postgres server running locally.
// Set the DATABASE_URL environment variable to point to your local Postgres instance.
// Make sure to use --test-threads=1 to avoid unexpected behavior.
// You can run the test using the following command:
//
// DATABASE_URL="postgres://postgres:postgres@localhost:5432" cargo test -- --nocapture --test-threads=1

use sqlx::PgPool;
use std::env;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use unisub::PubSub;

#[tokio::test]
async fn test_pub_sub_flow() {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    let pubsub = PubSub::new(pool).await.unwrap();

    pubsub.create_topic("test_topic").await.ok();

    // Subscribe to the topic and process new messages

    let mut pubsub2 = pubsub.clone();
    let handle = tokio::spawn(async move {
        pubsub2
            .subscribe("test_topic", move |message| {
                let message = message.clone();
                async move {
                    assert_eq!(message, b"Hello, world!".to_vec());
                    Ok(())
                }
            })
            .await
            .expect("Failed to subscribe to topic");
    });

    // Push a message to the topic

    pubsub
        .push("test_topic", b"Hello, world!")
        .await
        .expect("Failed to push message");

    // sleep for a second to allow the message to be processed
    sleep(Duration::from_secs(1)).await;

    // Shutdown the subscription

    println!("Sending shutdown signal...");
    pubsub.shutdown().await;
    println!("Shutdown signal sent.");

    // Wait for the subscription to finish
    handle.await.unwrap();
}

#[tokio::test]
async fn test_create_and_remove_topic() {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    let pubsub = PubSub::new(pool).await.unwrap();

    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database for verification");

    // Attempt to create a new topic
    pubsub
        .create_topic("temporary_topic")
        .await
        .expect("Failed to create topic");

    // Verify that the topic has been created
    let row_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM topics WHERE name = $1",
        "temporary_topic"
    )
    .fetch_one(&db)
    .await
    .expect("Failed to execute query to check topic");

    assert_eq!(row_count, Some(1), "Topic should be created");

    // Attempt to remove the topic
    pubsub
        .remove_topic("temporary_topic")
        .await
        .expect("Failed to remove topic");

    // Verify that the topic has been removed
    let row_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM topics WHERE name = $1",
        "temporary_topic"
    )
    .fetch_one(&db)
    .await
    .expect("Failed to execute query to check topic");

    assert_eq!(row_count, Some(0), "Topic should be removed");
}

#[tokio::test]
async fn test_message_order() {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&database_url).await.unwrap();
    let pubsub = PubSub::new(pool).await.unwrap();

    pubsub.create_topic("order_test_topic").await.ok();

    // Create an Arc-Mutex for thread-safe modification of received_messages vector
    let received_messages = Arc::new(Mutex::new(Vec::new()));

    // Clone objects for use inside the spawned thread
    let mut pubsub2 = pubsub.clone();
    let received_messages2 = received_messages.clone();

    // Spawn a new async thread for subscribing to the topic
    let handle = tokio::spawn(async move {
        pubsub2
            .subscribe("order_test_topic", move |message| {
                // Clone the Arc for use inside this specific iteration of the closure
                let received_messages = received_messages2.clone();
                let message = message.clone();
                async move {
                    received_messages.lock().await.push(message);
                    Ok(())
                }
            })
            .await
            .expect("Failed to subscribe to topic");
    });

    // Push messages to the topic
    pubsub
        .push("order_test_topic", b"1")
        .await
        .expect("Failed to push message");
    pubsub
        .push("order_test_topic", b"2")
        .await
        .expect("Failed to push message");

    // Give some time for messages to be processed
    sleep(Duration::from_secs(1)).await;

    // Wait for the subscription to finish
    pubsub.shutdown().await;
    handle.await.unwrap();

    // Lock the mutex and check received messages
    let received_messages = received_messages.lock().await;
    assert_eq!(*received_messages, vec![b"1".to_vec(), b"2".to_vec()]);
}
