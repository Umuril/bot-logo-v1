pub mod candle_vtracer;
pub mod claude_svg;
pub mod external;
pub mod local_llm_svg;
pub mod svg_prompt;
pub mod vtracer_wrapper;

use std::path::PathBuf;

pub struct GenerationRequest {
    pub prompt: String,
    pub reference_svg_path: Option<PathBuf>,
    pub reference_png_path: Option<PathBuf>,
}

pub trait Pipeline {
    fn generate(&self, request: &GenerationRequest) -> anyhow::Result<PathBuf>;
}

/// Output path for a text-based (LLM-authored) pipeline: `$TMPDIR/<subdir>/<slugified-prompt>.svg`.
/// Mirrors `CandleVtracerPipeline::output_paths`'s slugging convention.
pub(crate) fn svg_output_path(subdir: &str, prompt: &str) -> PathBuf {
    let slug: String = prompt.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).take(40).collect();
    std::env::temp_dir().join(subdir).join(format!("{slug}.svg"))
}

/// Runs `f` under `label`, printing how long it took. Mirrors `CandleVtracerPipeline`'s own
/// (private) timing helper so every pipeline's progress reads the same way on stdout.
pub(crate) fn timed<T>(label: &str, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    println!("{label}...");
    let start = std::time::Instant::now();
    let result = f();
    println!("{label} done in {:.2?}", start.elapsed());
    result
}
