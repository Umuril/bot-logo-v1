use super::{GenerationRequest, Pipeline};
use anyhow::{bail, Context};
use std::path::PathBuf;
use std::process::Command;

pub struct ExternalPipeline {
    pub command: Vec<String>,
}

impl Pipeline for ExternalPipeline {
    fn generate(&self, request: &GenerationRequest) -> anyhow::Result<PathBuf> {
        let Some((program, args)) = self.command.split_first() else {
            bail!("external pipeline command is empty");
        };

        let mut cmd = Command::new(program);
        cmd.args(args).arg(&request.prompt);
        if let Some(ref_svg) = &request.reference_svg_path {
            cmd.arg("--reference-svg").arg(ref_svg);
        }
        if let Some(ref_png) = &request.reference_png_path {
            cmd.arg("--reference-png").arg(ref_png);
        }

        let output = cmd.output().context("failed to run external generation command")?;
        if !output.status.success() {
            bail!("external generation command exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr));
        }

        let path_str = String::from_utf8(output.stdout).context("external command's stdout was not valid UTF-8")?;
        let path = PathBuf::from(path_str.trim());
        if !path.exists() {
            bail!("external command reported SVG path {:?} but it does not exist", path);
        }
        Ok(path)
    }
}
