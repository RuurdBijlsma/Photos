#![allow(clippy::missing_errors_doc)]

use color_eyre::eyre::Result;
use futures_util::StreamExt;
use language_model::{ChatEvent, ChatSession, LlamaClient};
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

pub async fn run() -> Result<()> {
    let client = LlamaClient::with_base_url("http://localhost:8080").build();
    let mut session = ChatSession::new(client.clone());

    let img_island = Path::new("media_dir/examples/island.png");
    let img_farm = Path::new("media_dir/examples/farm.png");
    let img_torus = Path::new("media_dir/examples/torus.png");
    let prompt = "Caption this image in one paragraph. Respond with the caption only.";

    let now = Instant::now();

    // Warmup
    info!(
        "Island: {}",
        client.chat(prompt).images(&[img_island]).call().await?
    );
    let now2 = Instant::now();
    // One-off prompts (no history)
    info!(
        "Farm: {}",
        client.chat(prompt).images(&[img_farm]).call().await?
    );
    info!(
        "Torus: {}",
        client.chat(prompt).images(&[img_torus]).call().await?
    );
    // Using Session (chat history is remembered)
    info!(
        "Island with session: {}",
        session.chat(prompt).images(&[img_island]).call().await?
    );
    info!(
        "Follow up: {}",
        session.chat("Where might this be?").call().await?
    );
    // Compare two images
    info!(
        "Similarities: {}",
        session
            .chat("What are the similarities with this picture?")
            .images(&[img_torus])
            .call()
            .await?
    );

    info!("Total time for [API]: {:?}", now.elapsed());
    info!(
        "Total time for [API] (excluding warmup): {:?}",
        now2.elapsed()
    );

    // Stream chat example
    let mut stream = session
        .chat_stream("how do i get minecraft mods like this?")
        .call()
        .await?;
    while let Some(event) = stream.next().await {
        if let ChatEvent::Content(c) = event? {
            print!("{c}");
            std::io::stdout().flush()?;
        }
    }
    println!();

    // Follow up on stream chat.
    info!(
        "Followup 3: {}",
        session.chat("can you expand on that?").call().await?
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    color_eyre::install()?;

    run().await?;

    Ok(())
}
