use language_model::LlamaClient;
use ml_analysis::get_quality_judgement;
use std::path::Path;
use std::time::Instant;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let llm_client = LlamaClient::with_base_url("http://localhost:8080").build();

    let images = vec![
        Path::new("media_dir/rutenl/tree.jpg"),
        Path::new("media_dir/rutenl/sunset.jpg"),
        Path::new("media_dir/rutenl/pics/PICT0017.JPG"),
    ];

    for image in images {
        let now = Instant::now();
        let quality_data = get_quality_judgement(&llm_client, image).await?;
        if let Some(qd) = quality_data {
            println!(
                "{} judged quality: {:?}, overall score: {}",
                image.file_name().unwrap().to_string_lossy(),
                qd,
                qd.weighted_score()
            );
            println!("\tget_quality_data {:?}", now.elapsed());
        } else {
            eprintln!("Couldn't get quality data");
        }
    }

    Ok(())
}
