use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeviceChoice {
    Cpu,
    Gpu,
}

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
        /// Which device to run inference on. Defaults to GPU (Metal); the CPU backend
        /// can't seed its RNG, so seeding is skipped when running on CPU.
        #[arg(long, value_enum, default_value = "gpu")]
        device: DeviceChoice,
    },
    /// Shell out to any external generation command
    External {
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    /// LLM-authored SVG via the `claude` CLI (uses your existing Claude subscription login)
    ClaudeSvg,
    /// LLM-authored SVG via a local candle-hosted model (Qwen2.5-Coder-7B-Instruct)
    LlmSvg {
        #[arg(long, value_enum, default_value = "gpu")]
        device: DeviceChoice,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
