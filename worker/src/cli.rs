use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "worker")]
pub struct Cli {
    #[command(subcommand)]
    pub pipeline: PipelineChoice,

    #[arg(long, global = true)]
    pub repeat: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum PipelineChoice {
    /// Built-in local Stable-Diffusion-family generation + vtracer vectorization
    CandleVtracer {
        #[arg(long)]
        model: Option<String>,
    },
    /// Shell out to any external generation command
    External {
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
