use super::svg_prompt::{build_svg_prompt, extract_svg, system_prompt};
use super::{svg_output_path, timed, GenerationRequest, Pipeline};
use anyhow::{bail, Context};
use std::path::PathBuf;
use std::process::Command;

/// LLM-authored SVG via the `claude` CLI in headless mode (`claude -p "..."`), using whatever
/// Claude login/subscription is already active on this machine — no API key needed.
pub struct ClaudeSvgPipeline;

impl Pipeline for ClaudeSvgPipeline {
    fn generate(&self, request: &GenerationRequest) -> anyhow::Result<PathBuf> {
        let full_prompt = format!("{}\n\n{}", system_prompt(), build_svg_prompt(request));

        let raw = timed("claude-svg: calling claude CLI", || {
            let output = Command::new("claude")
                .arg("-p")
                .arg(&full_prompt)
                .output()
                .context("failed to run the `claude` CLI \u{2014} is it installed and on PATH?")?;

            if !output.status.success() {
                bail!("claude CLI exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr));
            }

            String::from_utf8(output.stdout).context("claude CLI stdout was not valid UTF-8")
        })?;
        let svg = extract_svg(&raw)?;

        let svg_path = svg_output_path("worker-claude-svg", &request.prompt);
        std::fs::create_dir_all(svg_path.parent().unwrap())?;
        std::fs::write(&svg_path, svg)?;
        println!("claude-svg: wrote SVG to {}", svg_path.display());
        Ok(svg_path)
    }
}
