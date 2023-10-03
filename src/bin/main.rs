use std::env;

use clap::Parser;
use sqlx::PgPool;

use unisub::{Error, PubSub};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Args = Args::parse();

    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;
    let pubsub = PubSub::new(pool.clone()).await?;

    match args.cmd {
        Commands::Migrate => run_migrations(&pool).await?,
        Commands::AddTopic(args) => add_topic(&pubsub, args.topic_name).await?,
        Commands::RemoveTopic(args) => remove_topic(&pubsub, args.topic_name).await?,
    }

    Ok(())
}

async fn run_migrations(pool: &PgPool) -> Result<(), Error> {
    unisub::run_migrations(pool).await?;
    println!("Migrations completed successfully");
    Ok(())
}

async fn add_topic(pubsub: &PubSub, topic_name: String) -> Result<(), Error> {
    pubsub.create_topic(&topic_name).await?;
    println!("Topic '{}' created successfully", topic_name);
    Ok(())
}

async fn remove_topic(pubsub: &PubSub, topic_name: String) -> Result<(), Error> {
    pubsub.remove_topic(&topic_name).await?;
    println!("Topic '{}' removed successfully", topic_name);
    Ok(())
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Commands,
}

#[derive(Parser)]
enum Commands {
    Migrate,
    AddTopic(AddTopic),
    RemoveTopic(RemoveTopic),
}

#[derive(Parser)]
struct AddTopic {
    topic_name: String,
}

#[derive(Parser)]
struct RemoveTopic {
    topic_name: String,
}
