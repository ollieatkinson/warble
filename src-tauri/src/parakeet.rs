use anyhow::{anyhow, bail, Context, Result};
use parakeet_rs::{
    ExecutionConfig, ExecutionProvider as LibraryExecutionProvider, Parakeet,
    ParakeetTDT as LibraryParakeetTdt, TimestampMode, Transcriber,
};
use std::path::{Path, PathBuf};

use crate::runtime;
use crate::state::InferenceProvider;

pub const SAMPLE_RATE: u32 = 16_000;
pub const MODEL_ID: &str = "parakeet-tdt-0.6b-v3-int8";
pub const CTC_MODEL_ID: &str = "parakeet-ctc-0.6b";

const TDT_REQUIRED_FILES: &[&str] = &["vocab.txt"];
const TDT_ENCODER_CANDIDATES: &[&str] = &[
    "encoder-model.onnx",
    "encoder-model.int8.onnx",
    "encoder.onnx",
];
const TDT_DECODER_CANDIDATES: &[&str] = &[
    "decoder_joint-model.onnx",
    "decoder_joint-model.int8.onnx",
    "decoder_joint.onnx",
    "decoder-model.onnx",
];
const CTC_MODEL_CANDIDATES: &[&str] = &[
    "model.onnx",
    "model_fp16.onnx",
    "model_int8.onnx",
    "model_q4.onnx",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptionFamily {
    Tdt,
    Ctc,
}

pub struct ParakeetTdt {
    runtime: LibraryParakeetTdt,
    provider: InferenceProvider,
}

impl ParakeetTdt {
    pub fn load(model_root: &Path) -> Result<Self> {
        let model_dir = model_root.join(MODEL_ID);
        Self::load_from_dir(&model_dir)
    }

    pub fn load_from_dir(model_dir: &Path) -> Result<Self> {
        if !model_ready_in_dir(model_dir) {
            bail!("Parakeet TDT model is missing at {}", model_dir.display());
        }

        runtime::ensure_ort_initialized()?;

        let preferred_provider = preferred_inference_provider();
        let (provider, runtime) =
            load_with_directml_fallback(preferred_provider, |use_directml| {
                LibraryParakeetTdt::from_pretrained(model_dir, Some(execution_config(use_directml)))
                    .map_err(|error| anyhow!("failed to load Parakeet TDT runtime: {error}"))
            })?;

        Ok(Self { runtime, provider })
    }

    pub fn transcribe_audio(&mut self, audio: &[f32]) -> Result<String> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let result = self
            .runtime
            .transcribe_samples(
                audio.to_vec(),
                SAMPLE_RATE,
                1,
                Some(TimestampMode::Sentences),
            )
            .map_err(|error| anyhow!("Parakeet TDT transcription failed: {error}"))?;

        Ok(normalize_output_text(&result.text))
    }

    pub fn transcribe_wav_path(&mut self, wav_path: &Path) -> Result<String> {
        let audio = read_wav_mono(wav_path)?;
        self.transcribe_audio(&audio)
    }

    pub(crate) fn provider(&self) -> InferenceProvider {
        self.provider
    }
}

pub struct ParakeetCtc {
    runtime: Parakeet,
    provider: InferenceProvider,
}

impl ParakeetCtc {
    pub fn load(model_root: &Path) -> Result<Self> {
        let model_dir = model_root.join(CTC_MODEL_ID);
        Self::load_from_dir(&model_dir)
    }

    pub fn load_from_dir(model_dir: &Path) -> Result<Self> {
        if !ctc_model_ready_in_dir(model_dir) {
            bail!("Parakeet CTC model is missing at {}", model_dir.display());
        }

        runtime::ensure_ort_initialized()?;

        let preferred_provider = preferred_inference_provider();
        let (provider, runtime) =
            load_with_directml_fallback(preferred_provider, |use_directml| {
                Parakeet::from_pretrained(model_dir, Some(execution_config(use_directml)))
                    .map_err(|error| anyhow!("failed to load Parakeet CTC runtime: {error}"))
            })?;

        Ok(Self { runtime, provider })
    }

    pub fn transcribe_audio(&mut self, audio: &[f32]) -> Result<String> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let result = self
            .runtime
            .transcribe_samples(audio.to_vec(), SAMPLE_RATE, 1, Some(TimestampMode::Words))
            .map_err(|error| anyhow!("Parakeet CTC transcription failed: {error}"))?;

        Ok(normalize_output_text(&result.text))
    }

    pub(crate) fn provider(&self) -> InferenceProvider {
        self.provider
    }
}

pub fn model_ready_at(root: &Path) -> bool {
    model_ready_in_dir(&root.join(MODEL_ID))
}

pub fn model_ready_in_dir(model_dir: &Path) -> bool {
    TDT_REQUIRED_FILES
        .iter()
        .all(|filename| model_dir.join(filename).exists())
        && has_any_file(model_dir, TDT_ENCODER_CANDIDATES)
        && has_any_file(model_dir, TDT_DECODER_CANDIDATES)
}

pub fn ctc_model_ready_at(root: &Path) -> bool {
    ctc_model_ready_in_dir(&root.join(CTC_MODEL_ID))
}

pub fn ctc_model_ready_in_dir(model_dir: &Path) -> bool {
    model_dir.join("tokenizer.json").exists() && has_any_file(model_dir, CTC_MODEL_CANDIDATES)
}

pub fn detect_model_dir(root: &Path) -> Option<(TranscriptionFamily, PathBuf)> {
    let tdt_dir = root.join(MODEL_ID);
    if model_ready_in_dir(&tdt_dir) {
        return Some((TranscriptionFamily::Tdt, tdt_dir));
    }

    let ctc_dir = root.join(CTC_MODEL_ID);
    if ctc_model_ready_in_dir(&ctc_dir) {
        return Some((TranscriptionFamily::Ctc, ctc_dir));
    }

    if model_ready_in_dir(root) {
        return Some((TranscriptionFamily::Tdt, root.to_path_buf()));
    }

    if ctc_model_ready_in_dir(root) {
        return Some((TranscriptionFamily::Ctc, root.to_path_buf()));
    }

    None
}

fn has_any_file(dir: &Path, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|filename| dir.join(filename).exists())
}

fn load_with_directml_fallback<T>(
    preferred_provider: InferenceProvider,
    loader: impl Fn(bool) -> Result<T>,
) -> Result<(InferenceProvider, T)> {
    if preferred_provider == InferenceProvider::Directml {
        match loader(true) {
            Ok(runtime) => return Ok((InferenceProvider::Directml, runtime)),
            Err(directml_error) => {
                let runtime = loader(false)
                    .with_context(|| format!("{directml_error}; falling back to CPU"))?;
                return Ok((InferenceProvider::Cpu, runtime));
            }
        }
    }

    Ok((InferenceProvider::Cpu, loader(false)?))
}

fn execution_config(directml_enabled: bool) -> ExecutionConfig {
    let provider = if directml_enabled {
        #[cfg(target_os = "windows")]
        {
            LibraryExecutionProvider::DirectML
        }
        #[cfg(not(target_os = "windows"))]
        {
            LibraryExecutionProvider::Cpu
        }
    } else {
        LibraryExecutionProvider::Cpu
    };

    let config = ExecutionConfig::new()
        .with_execution_provider(provider)
        .with_intra_threads(4)
        .with_inter_threads(1);

    config.with_custom_configure(move |builder| {
        let builder = builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?;
        if directml_enabled {
            Ok(builder
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?)
        } else {
            Ok(builder)
        }
    })
}

fn preferred_inference_provider() -> InferenceProvider {
    #[cfg(target_os = "windows")]
    if directml_runtime_available() {
        return InferenceProvider::Directml;
    }

    InferenceProvider::Cpu
}

#[cfg(target_os = "windows")]
fn directml_runtime_available() -> bool {
    runtime::directml_runtime_available()
}

fn normalize_output_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn resample_to_16khz(input: &[f32], from_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == SAMPLE_RATE {
        return input.to_vec();
    }

    let output_len =
        ((input.len() as f64) * SAMPLE_RATE as f64 / from_rate as f64).round() as usize;
    let output_len = output_len.max(1);
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let source_position = (index as f64) * (from_rate as f64) / SAMPLE_RATE as f64;
        let left = source_position.floor() as usize;
        let right = (left + 1).min(input.len().saturating_sub(1));
        let weight = source_position - left as f64;
        let left_sample = input.get(left).copied().unwrap_or(0.0);
        let right_sample = input.get(right).copied().unwrap_or(left_sample);
        output.push((left_sample as f64 * (1.0 - weight) + right_sample as f64 * weight) as f32);
    }

    output
}

fn read_wav_mono(path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));

    let mono = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => {
            let samples: Vec<i16> = reader
                .samples::<i16>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("failed to read PCM16 audio")?;
            samples
                .chunks(channels)
                .map(|frame| {
                    let sum = frame
                        .iter()
                        .map(|sample| *sample as f32 / i16::MAX as f32)
                        .sum::<f32>();
                    sum / channels as f32
                })
                .collect()
        }
        (hound::SampleFormat::Float, 32) => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("failed to read float WAV audio")?;
            samples
                .chunks(channels)
                .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
                .collect()
        }
        _ => bail!("unsupported WAV format"),
    };

    if spec.sample_rate == SAMPLE_RATE {
        return Ok(mono);
    }

    Ok(resample_to_16khz(&mono, spec.sample_rate))
}
