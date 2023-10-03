//! A Pub/Sub implementation using Postgres as the backend.
//!
//! This crate provides functionalities to subscribe and publish messages
//! to different topics.

use sqlx::postgres::PgListener;
use sqlx::PgPool;
use std::future::Future;
use thiserror::Error;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

/// A Pub/Sub struct for message interactions.
#[derive(Clone)]
pub struct PubSub {
    pool: PgPool,
    shutdown: CancellationToken,
}

impl PubSub {
    /// Create a new `PubSub` instance.
    ///
    /// # Arguments
    ///
    /// * `database_url` - A string slice that holds the database URL.
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        Ok(Self {
            pool,
            shutdown: CancellationToken::new(),
        })
    }

    /// Shutdown the `PubSub` system.
    ///
    /// This function triggers the cancellation token to stop all subscribers.
    pub async fn shutdown(&self) {
        self.shutdown.cancel();
    }

    /// Create a new topic.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the name of the topic.
    pub async fn create_topic(&self, name: &str) -> Result<(), Error> {
        sqlx::query!(r#"INSERT INTO topics (name) VALUES ($1)"#, name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove a topic.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the name of the topic.
    pub async fn remove_topic(&self, name: &str) -> Result<(), Error> {
        sqlx::query!("DELETE FROM topics WHERE name = $1", name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Publish a message to a topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to publish the message to.
    /// * `content` - The content of the message as a byte vector.
    pub async fn push(&self, topic: &str, content: &[u8]) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO messages (topic_id, content)
            SELECT id, $2
            FROM topics WHERE name = $1
            "#,
            topic,
            content
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Subscribe to a topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to subscribe to.
    /// * `callback` - The function to call when a message arrives.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the callback function.
    /// * `Fut` - The type of the future that the callback returns.
    pub async fn subscribe<F, Fut>(&mut self, topic: &str, callback: F) -> Result<(), Error>
    where
        F: FnMut(Vec<u8>) -> Fut + Clone,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        // First, listen for new messages from postgres so we don't miss anything
        let mut listener = PgListener::connect_with(&self.pool).await?;
        listener.listen("new_message").await?;
        let mut stream = listener.into_stream();

        // Drain the existing messages from the queue
        let id_rows = sqlx::query!(
            r#"
            SELECT messages.id
            FROM messages, topics
            WHERE
                topics.name = $1 AND
                messages.topic_id = topics.id AND
                messages.status = 'new'
            ORDER BY messages.published_at ASC
            "#,
            topic
        )
        .fetch_all(&self.pool)
        .await?;

        // Process each message individually
        for row in id_rows {
            let mut tx = self.pool.begin().await?;
            let message = sqlx::query!(
                r#"
                SELECT messages.content
                FROM messages
                WHERE messages.id = $1
                FOR UPDATE SKIP LOCKED
                "#,
                row.id
            )
            .fetch_one(&mut *tx)
            .await?;

            process_message(&mut tx, row.id, message.content, callback.clone()).await?;
            tx.commit().await?;
        }

        loop {
            tokio::select! {
                // If the shutdown token is cancelled, break out of the loop
                _ = self.shutdown.cancelled() => {
                    break;
                }
                // If the listener receives a notification, process the message
                notification = stream.next() => {
                    if let Some(Ok(notification)) = notification {
                        if notification.channel() == "new_message" {
                            let message_id: i32 = notification.payload().parse()?;
                            let mut tx = self.pool.begin().await?;
                            let row = sqlx::query!(
                                r#"
                                SELECT messages.content
                                FROM messages, topics
                                WHERE
                                    messages.id = $1 AND
                                    topics.name = $2 AND
                                    messages.topic_id = topics.id AND
                                    messages.status = 'new'
                                LIMIT 1
                                "#,
                                message_id,
                                topic
                            )
                            .fetch_one(&mut *tx)
                            .await?;

                            sqlx::query!(
                                "UPDATE messages SET status = 'processing' WHERE id = $1",
                                message_id
                            )
                            .execute(&mut *tx)
                            .await?;

                            process_message(&mut tx, message_id, row.content, callback.clone()).await?;
                            tx.commit().await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for PubSub {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

async fn process_message<'a, F, Fut>(
    tx: &'a mut sqlx::Transaction<'static, sqlx::Postgres>,
    message_id: i32,
    message_content: Vec<u8>,
    mut callback: F,
) -> Result<(), Error>
where
    F: FnMut(Vec<u8>) -> Fut,
    Fut: Future<Output = Result<(), Error>> + Send + 'static,
{
    callback(message_content).await?;
    sqlx::query!(
        "UPDATE messages SET status = 'processed' WHERE id = $1",
        message_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Errors that can occur in the Pub/Sub system.
#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
    #[error("parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

/// Run database migrations.
///
/// # Arguments
///
/// * `db` - A `PgPool` reference for database interaction.
pub async fn run_migrations(db: &PgPool) -> Result<(), Error> {
    sqlx::migrate!("./migrations").run(db).await?;
    Ok(())
}
