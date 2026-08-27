use super::vtracer_wrapper::png_to_svg;
use super::{GenerationRequest, Pipeline};
use crate::cli::DeviceChoice;
use anyhow::{Context, Result};
use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_transformers::models::stable_diffusion::{self, StableDiffusionConfig};
use hf_hub::HFClientSync;
use std::path::PathBuf;
use tokenizers::Tokenizer;

const DEFAULT_MODEL: &str = "stabilityai/sdxl-turbo";
const TOKENIZER_REPO: &str = "openai/clip-vit-large-patch14";
const TOKENIZER2_REPO: &str = "laion/CLIP-ViT-bigG-14-laion2B-39B-b160k";

pub struct CandleVtracerPipeline {
    pub model: String,
    pub device: DeviceChoice,
}

impl CandleVtracerPipeline {
    pub fn new(model: Option<String>, device: DeviceChoice) -> Self {
        CandleVtracerPipeline { model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()), device }
    }

    fn output_paths(&self, prompt: &str) -> (PathBuf, PathBuf) {
        let slug: String = prompt
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .take(40)
            .collect();
        let dir = std::env::temp_dir().join("worker-candle-vtracer");
        (dir.join(format!("{slug}.png")), dir.join(format!("{slug}.svg")))
    }

    /// Real Stable Diffusion XL Turbo inference, following candle-transformers' own
    /// stable-diffusion example (candle-examples/examples/stable-diffusion/main.rs upstream),
    /// trimmed to a single text-to-image pass: no img2img/inpainting/CLI args/tracing/guidance
    /// (Turbo's own guidance_scale is 0, so the unconditional-prompt branch never applies).
    /// Downloads weights via hf-hub on first run.
    ///
    /// Only `stabilityai/sdxl-turbo` (the default) is wired up right now. candle maps each
    /// Stable Diffusion version to its own hardcoded set of component file paths (see
    /// `ModelFile`/`StableDiffusionVersion` in the upstream example) rather than accepting an
    /// arbitrary HF repo ID, so supporting other checkpoints via `self.model` would mean
    /// replicating that per-version file mapping, not just swapping a string. Left as a known
    /// follow-up rather than implemented speculatively.
    fn run_inference(&self, prompt: &str, png_path: &std::path::Path) -> Result<()> {
        if self.model != DEFAULT_MODEL {
            anyhow::bail!("only {DEFAULT_MODEL} is wired up for real generation right now; got {:?}", self.model);
        }

        std::fs::create_dir_all(png_path.parent().unwrap())?;

        let device = match self.device {
            DeviceChoice::Cpu => Device::Cpu,
            DeviceChoice::Gpu => Device::new_metal(0).context(
                "failed to initialize the Metal GPU device; pass --device cpu to run on CPU instead \
                 (rebuild with `--features metal` if this binary wasn't built with GPU support)",
            )?,
        };
        let dtype = DType::F32;
        let hf_client = HFClientSync::new()?;

        let sd_config = StableDiffusionConfig::sdxl_turbo(None, None, None);

        println!("candle: building CLIP text embeddings");
        let text_embeddings = {
            let first = Self::clip_embeddings(
                &hf_client,
                prompt,
                TOKENIZER_REPO,
                "text_encoder/model.safetensors",
                &sd_config.clip,
                &device,
                dtype,
            )?;
            let clip2_config = sd_config.clip2.as_ref().context("sdxl_turbo config is missing its second CLIP config")?;
            let second = Self::clip_embeddings(
                &hf_client,
                prompt,
                TOKENIZER2_REPO,
                "text_encoder_2/model.safetensors",
                clip2_config,
                &device,
                dtype,
            )?;
            Tensor::cat(&[first, second], candle_core::D::Minus1)?
        };

        println!("candle: building VAE");
        let vae_weights = Self::hf_download(&hf_client, DEFAULT_MODEL, "vae/diffusion_pytorch_model.safetensors")?;
        let vae = sd_config.build_vae(vae_weights, &device, dtype)?;

        println!("candle: building UNet");
        let unet_weights = Self::hf_download(&hf_client, DEFAULT_MODEL, "unet/diffusion_pytorch_model.safetensors")?;
        let unet = sd_config.build_unet(unet_weights, &device, 4, false, dtype)?;

        let n_steps = 1; // candle's own default step count for Turbo
        let mut scheduler = sd_config.build_scheduler(n_steps)?;
        // The CPU backend can't seed its RNG (candle_core::cpu_backend always errors on
        // set_seed), so runs on CPU are non-deterministic; GPU backends do support it.
        if !device.is_cpu() {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
            device.set_seed(seed)?;
        }

        let vae_scale = 0.13025; // candle's own constant for the Turbo/XL VAE

        let latents = Tensor::randn(0f32, 1f32, (1, 4, sd_config.height / 8, sd_config.width / 8), &device)?;
        let mut latents = (latents * scheduler.init_noise_sigma())?.to_dtype(dtype)?;

        println!("candle: sampling");
        for &timestep in scheduler.timesteps().to_vec().iter() {
            let latent_model_input = scheduler.scale_model_input(latents.clone(), timestep)?;
            let noise_pred = unet.forward(&latent_model_input, timestep as f64, &text_embeddings)?;
            latents = scheduler.step(&noise_pred, timestep, &latents)?;
        }

        println!("candle: decoding and saving PNG");
        let images = vae.decode(&(latents / vae_scale)?)?;
        let images = ((images / 2.)? + 0.5)?.to_device(&Device::Cpu)?;
        let images = (images.clamp(0f32, 1.)? * 255.)?.to_dtype(DType::U8)?;
        let image_tensor = images.i(0)?;
        Self::save_png(&image_tensor, png_path)?;

        Ok(())
    }

    /// Downloads `filename` from `repo_id` (an "owner/name" HF repo id) into the local cache.
    fn hf_download(client: &HFClientSync, repo_id: &str, filename: &str) -> Result<PathBuf> {
        let (owner, name) = repo_id.split_once('/').with_context(|| format!("repo id {repo_id:?} is not in owner/name form"))?;
        Ok(client.model(owner, name).download_file().filename(filename).send()?)
    }

    fn clip_embeddings(
        hf_client: &HFClientSync,
        prompt: &str,
        tokenizer_repo: &str,
        clip_weights_file: &str,
        clip_config: &stable_diffusion::clip::Config,
        device: &Device,
        dtype: DType,
    ) -> Result<Tensor> {
        let tokenizer_path = Self::hf_download(hf_client, tokenizer_repo, "tokenizer.json")?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;
        let pad_id = match &clip_config.pad_with {
            Some(padding) => *tokenizer.get_vocab(true).get(padding.as_str()).context("pad token missing from vocab")?,
            None => *tokenizer.get_vocab(true).get("<|endoftext|>").context("<|endoftext|> missing from vocab")?,
        };

        let mut tokens = tokenizer.encode(prompt, true).map_err(anyhow::Error::msg)?.get_ids().to_vec();
        if tokens.len() > clip_config.max_position_embeddings {
            anyhow::bail!("prompt is too long: {} tokens > {} max", tokens.len(), clip_config.max_position_embeddings);
        }
        while tokens.len() < clip_config.max_position_embeddings {
            tokens.push(pad_id);
        }
        let tokens = Tensor::new(tokens.as_slice(), device)?.unsqueeze(0)?;

        let clip_weights = Self::hf_download(hf_client, DEFAULT_MODEL, clip_weights_file)?;
        let text_model = stable_diffusion::build_clip_transformer(clip_config, clip_weights, device, DType::F32)?;
        let embeddings = text_model.forward(&tokens)?;
        Ok(embeddings.to_dtype(dtype)?)
    }

    fn save_png(tensor: &Tensor, path: &std::path::Path) -> Result<()> {
        let (channels, height, width) = tensor.dims3()?;
        anyhow::ensure!(channels == 3, "expected a 3-channel image tensor, got {channels}");
        let pixels = tensor.permute((1, 2, 0))?.contiguous()?.flatten_all()?.to_vec1::<u8>()?;
        let img: image::RgbImage =
            image::ImageBuffer::from_raw(width as u32, height as u32, pixels).context("failed to build image buffer from tensor data")?;
        img.save(path)?;
        Ok(())
    }
}

impl Pipeline for CandleVtracerPipeline {
    fn generate(&self, request: &GenerationRequest) -> anyhow::Result<PathBuf> {
        let (png_path, svg_path) = self.output_paths(&request.prompt);
        self.run_inference(&request.prompt, &png_path)?;
        png_to_svg(&png_path, &svg_path)?;
        Ok(svg_path)
    }
}
