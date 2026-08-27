mod cli;
mod client;
mod config;
mod pipeline;
mod repeat;
mod review;

use cli::{Cli, PipelineChoice};
use pipeline::{candle_vtracer::CandleVtracerPipeline, external::ExternalPipeline, GenerationRequest, Pipeline};
use review::Decision;
use shared::SubmitRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse_args();
    dotenvy::from_filename("worker.env").ok();
    let config = config::Config::from_env()?;
    let bot_client = client::BotClient::new(&config);

    let context = bot_client.fetch_context().await?;

    let pipeline: Box<dyn Pipeline> = match &cli.pipeline {
        PipelineChoice::CandleVtracer { model } => Box::new(CandleVtracerPipeline::new(model.clone())),
        PipelineChoice::External { command } => Box::new(ExternalPipeline { command: command.clone() }),
    };

    let (prompt, variant_of, reference_svg_path, reference_png_path) = if let Some(short_name) = &cli.repeat {
        let candidate = repeat::find_candidate(&context, short_name)
            .ok_or_else(|| anyhow::anyhow!("no candidate named {short_name:?} in current context"))?;

        let reference_dir = std::env::temp_dir().join("worker-repeat");
        std::fs::create_dir_all(&reference_dir)?;
        let reference_svg_path = reference_dir.join(format!("{short_name}.svg"));
        let reference_png_path = reference_dir.join(format!("{short_name}.png"));
        std::fs::write(&reference_svg_path, bot_client.download(&candidate.svg_url).await?)?;
        std::fs::write(&reference_png_path, bot_client.download(&candidate.png_url).await?)?;

        (candidate.prompt.clone(), Some(short_name.clone()), Some(reference_svg_path), Some(reference_png_path))
    } else {
        println!("Enter a prompt for the new candidate:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        (input.trim().to_string(), None, None, None)
    };

    let pipeline_name = match &cli.pipeline {
        PipelineChoice::CandleVtracer { .. } => "candle-vtracer",
        PipelineChoice::External { .. } => "external",
    };
    let model_name = match &cli.pipeline {
        PipelineChoice::CandleVtracer { model } => model.clone().unwrap_or_else(|| "stabilityai/sdxl-turbo".to_string()),
        PipelineChoice::External { command } => command.join(" "),
    };

    loop {
        let request = GenerationRequest {
            prompt: prompt.clone(),
            reference_svg_path: reference_svg_path.clone(),
            reference_png_path: reference_png_path.clone(),
        };
        let svg_path = pipeline.generate(&request)?;

        match review::prompt_for_decision(&svg_path)? {
            Decision::Accept => {
                let svg_content = std::fs::read_to_string(&svg_path)?;
                let submit_request = SubmitRequest {
                    svg: svg_content,
                    prompt: prompt.clone(),
                    pipeline: pipeline_name.to_string(),
                    model: model_name.clone(),
                    variant_of: variant_of.clone(),
                };
                let response = bot_client.submit(&submit_request).await?;
                println!("Submitted as {} (Discord message {})", response.short_name, response.message_id);
                break;
            }
            Decision::Retry => continue,
            Decision::Abandon => {
                println!("Abandoned — nothing submitted.");
                break;
            }
        }
    }

    Ok(())
}
