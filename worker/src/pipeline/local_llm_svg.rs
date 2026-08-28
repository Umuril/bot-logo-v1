use super::svg_prompt::{build_svg_prompt, extract_svg, system_prompt};
use super::{svg_output_path, timed, GenerationRequest, Pipeline};
use crate::cli::DeviceChoice;
use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::qwen2::{Config, ModelForCausalLM};
use hf_hub::HFClientSync;
use std::path::PathBuf;
use tokenizers::Tokenizer;

const MODEL_OWNER: &str = "Qwen";
const MODEL_NAME: &str = "Qwen2.5-Coder-7B-Instruct";
const WEIGHT_SHARDS: &[&str] = &[
    "model-00001-of-00004.safetensors",
    "model-00002-of-00004.safetensors",
    "model-00003-of-00004.safetensors",
    "model-00004-of-00004.safetensors",
];
const MAX_NEW_TOKENS: usize = 4096;
const GENERATION_HEARTBEAT_TOKENS: usize = 128;

/// LLM-authored SVG via a local Qwen2.5-Coder-7B-Instruct model, run entirely through candle
/// (no network dependency beyond the one-time weight download via hf-hub).
pub struct LocalLlmSvgPipeline {
    pub device: DeviceChoice,
}

impl LocalLlmSvgPipeline {
    pub fn new(device: DeviceChoice) -> Self {
        Self { device }
    }

    fn download(hf_client: &HFClientSync, filename: &str) -> Result<PathBuf> {
        Ok(hf_client.model(MODEL_OWNER, MODEL_NAME).download_file().filename(filename).send()?)
    }
}

impl Pipeline for LocalLlmSvgPipeline {
    fn generate(&self, request: &GenerationRequest) -> Result<PathBuf> {
        let device = match self.device {
            DeviceChoice::Cpu => Device::Cpu,
            DeviceChoice::Gpu => Device::new_metal(0).context(
                "failed to initialize the Metal GPU device; pass --device cpu to run on CPU instead \
                 (rebuild with `--features metal` if this binary wasn't built with GPU support)",
            )?,
        };
        let dtype = DType::F32;
        let hf_client = HFClientSync::new()?;

        let config: Config = timed("qwen: fetching config", || {
            let config_path = Self::download(&hf_client, "config.json")?;
            Ok(serde_json::from_str(&std::fs::read_to_string(config_path)?)?)
        })?;

        let tokenizer = timed("qwen: fetching tokenizer", || {
            let tokenizer_path = Self::download(&hf_client, "tokenizer.json")?;
            Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)
        })?;

        let weight_paths = timed(&format!("qwen: fetching model weights ({} shards)", WEIGHT_SHARDS.len()), || {
            WEIGHT_SHARDS.iter().map(|filename| Self::download(&hf_client, filename)).collect::<Result<Vec<_>>>()
        })?;

        let mut model = timed("qwen: loading model into memory", || {
            // Safety: the paths above were just resolved (and, on a cache hit, verified to exist) by hf-hub.
            let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_paths, dtype, &device)? };
            ModelForCausalLM::new(&config, vb).map_err(Into::into)
        })?;

        let prompt_text = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system_prompt(),
            build_svg_prompt(request)
        );
        let eos_token = tokenizer.token_to_id("<|im_end|>").context("tokenizer is missing the <|im_end|> token")?;
        let mut tokens = tokenizer.encode(prompt_text, true).map_err(anyhow::Error::msg)?.get_ids().to_vec();
        let start_gen = tokens.len();
        println!("qwen: prompt is {start_gen} tokens; generating (up to {MAX_NEW_TOKENS} new tokens)...");

        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let mut logits_processor = LogitsProcessor::new(seed, Some(0.7), Some(0.9));

        let generation_start = std::time::Instant::now();
        let mut stopped_at_eos = false;
        for index in 0..MAX_NEW_TOKENS {
            let context_size = if index == 0 { tokens.len() } else { 1 };
            let start_pos = tokens.len() - context_size;
            let input = Tensor::new(&tokens[start_pos..], &device)?.unsqueeze(0)?;
            let logits = model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;
            let next_token = logits_processor.sample(&logits)?;
            tokens.push(next_token);
            if next_token == eos_token {
                stopped_at_eos = true;
                break;
            }
            let generated_so_far = index + 1;
            if generated_so_far % GENERATION_HEARTBEAT_TOKENS == 0 {
                let elapsed = generation_start.elapsed();
                let tok_per_sec = generated_so_far as f64 / elapsed.as_secs_f64();
                println!("qwen: generated {generated_so_far} tokens in {elapsed:.2?} ({tok_per_sec:.1} tok/s)...");
            }
        }
        let new_token_count = tokens.len() - start_gen;
        let stop_reason = if stopped_at_eos { "hit end-of-message token" } else { "hit MAX_NEW_TOKENS limit" };
        println!("qwen: generation done in {:.2?}: {new_token_count} tokens ({stop_reason})", generation_start.elapsed());

        let generated = tokenizer.decode(&tokens[start_gen..], true).map_err(anyhow::Error::msg)?;
        let svg = extract_svg(&generated)?;

        let svg_path = svg_output_path("worker-llm-svg", &request.prompt);
        std::fs::create_dir_all(svg_path.parent().unwrap())?;
        std::fs::write(&svg_path, svg)?;
        println!("qwen: wrote SVG to {}", svg_path.display());
        Ok(svg_path)
    }
}
