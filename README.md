# Unisub

[![Crates.io](https://img.shields.io/crates/v/unisub.svg)](https://crates.io/crates/unisub) [![Docs.rs](https://docs.rs/unisub/badge.svg)](https://docs.rs/unisub)

Unisub is a Pub/Sub library for Rust, using Postgres as the backend. It offers a convenient way to publish and subscribe to messages across different topics.

## Features

- Publish messages to topics
- Subscribe to topics to receive messages
- Create and remove topics
- Includes a convenience binary for managing topics and running migrations
- Asynchronous design using Tokio

## Installation

Add Unisub as a dependency in your `Cargo.toml`:

```toml
[dependencies]
unisub = "*"
```

## Setting up the Postgres Environment

1. Install Postgres if you haven't already. You can download it from [here](https://www.postgresql.org/download/).
2. Create a new database and note the connection URL.
3. Make sure your Postgres database is accessible and running.

### Environment Variable

Set an environment variable called `DATABASE_URL` with the Postgres connection URL.

```bash
export DATABASE_URL=postgres://username:password@localhost/dbname
```

### Running Migrations and Setting Up Topics

#### Using CLI

First, run the migrations:

```bash
unisub migrate
```

Then you can add your topics:

```bash
unisub add-topic my_topic
```

#### Programmatically

```rust
use unisub::PubSub;
use unisub::migrate;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), unisub::Error> {
    let pool = PgPool::connect("your_database_url").await?;
    migrate(&pool).await?;
    let mut pubsub = PubSub::new(pool).await?;
    pubsub.add_topic("my_topic").await?;
    Ok(())
}
```

## Usage

### Library

```rust
use unisub::PubSub;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), unisub::Error> {
    let pool = PgPool::connect("your_database_url").await?;
    let mut pubsub = PubSub::new(pool).await?;
    pubsub.push("my_topic", b"My message").await?;
    Ok(())
}
```

And here's an example of how to subscribe to a topic:

```rust
use unisub::PubSub;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), unisub::Error> {
    // Connect to the database
    let pool = PgPool::connect("your_database_url").await?;
    let mut pubsub = PubSub::new(pool).await?;

    // Subscribe to the topic
    pubsub.subscribe("my_topic", |message| {
        async {
            // Print the received message
            println!("Received message: {:?}", message);

            // Simulate some asynchronous work with the received message
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("Finished processing message: {:?}", message);

            Ok(())
        }
    }).await?;

    Ok(())
}
```

## Contributing

Contributions are welcome! Please submit a pull request or create an issue to get started.

## License

This project is licensed under the MIT License.
