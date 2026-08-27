pub mod candle_vtracer;
pub mod external;
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
