use anyhow::{anyhow, Context, Result};
use parakeet_rs::{Nemotron, ParakeetEOU};
use std::path::{Path, PathBuf};

use crate::inference;
use crate::runtime;
use crate::state::{LivePreviewModelPreference, Settings, SystemProfile};

const EOU_MODEL_ID: &str = "parakeet-eou";
const NEMOTRON_MODEL_ID: &str = "nemotron-streaming";
const EOU_CHUNK_SAMPLES: usize = 2_560;
const NEMOTRON_CHUNK_SAMPLES: usize = 8_960;

const EOU_REQUIRED_FILES: &[&str] = &["encoder.onnx", "decoder_joint.onnx", "tokenizer.json"];
const NEMOTRON_REQUIRED_FILES: &[&str] = &[
    "encoder.onnx",
    "encoder.onnx.data",
    "decoder_joint.onnx",
    "tokenizer.model",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamingPreviewBackend {
    Nemotron,
    Eou,
}

impl StreamingPreviewBackend {
    fn diagnostic_backend(self) -> &'static str {
        match self {
            Self::Nemotron => "streaming-nemotron",
            Self::Eou => "streaming-eou",
        }
    }

    fn trying_detail(self) -> &'static str {
        match self {
            Self::Nemotron => "Using Nemotron Streaming first",
            Self::Eou => "Using Parakeet Realtime EOU first",
        }
    }

    fn active_detail(self) -> &'static str {
        match self {
            Self::Nemotron => "Nemotron Streaming is emitting text",
            Self::Eou => "Realtime EOU is emitting text",
        }
    }

    fn chunk_size(self) -> usize {
        match self {
            Self::Nemotron => NEMOTRON_CHUNK_SAMPLES,
            Self::Eou => EOU_CHUNK_SAMPLES,
        }
    }
}

pub(crate) struct StreamingPreviewConfig {
    backend: StreamingPreviewBackend,
    model_path: PathBuf,
    preferred_provider: crate::state::InferenceProvider,
}

impl StreamingPreviewConfig {
    pub(crate) fn diagnostic_backend(&self) -> &'static str {
        self.backend.diagnostic_backend()
    }

    pub(crate) fn trying_detail(&self) -> &'static str {
        self.backend.trying_detail()
    }
}

pub(crate) fn resolve_streaming_preview_config(
    settings: &Settings,
    system_profile: &SystemProfile,
) -> Option<StreamingPreviewConfig> {
    let preferred_provider = system_profile
        .supported_acceleration_providers
        .first()
        .copied()
        .unwrap_or(crate::state::InferenceProvider::Cpu);
    let preferred_order = match settings.live_preview_model {
        LivePreviewModelPreference::Auto | LivePreviewModelPreference::NemotronStreaming => [
            StreamingPreviewBackend::Nemotron,
            StreamingPreviewBackend::Eou,
        ],
        LivePreviewModelPreference::ParakeetEou => [
            StreamingPreviewBackend::Eou,
            StreamingPreviewBackend::Nemotron,
        ],
    };

    for backend in preferred_order {
        if let Some(config) = ready_streaming_preview_config(settings, preferred_provider, backend)
        {
            return Some(config);
        }
    }

    None
}

fn ready_streaming_preview_config(
    settings: &Settings,
    preferred_provider: crate::state::InferenceProvider,
    backend: StreamingPreviewBackend,
) -> Option<StreamingPreviewConfig> {
    let model_id = match backend {
        StreamingPreviewBackend::Nemotron => NEMOTRON_MODEL_ID,
        StreamingPreviewBackend::Eou => EOU_MODEL_ID,
    };
    let model_path = PathBuf::from(settings.installed_model_paths.get(model_id)?.as_str());
    let ready = match backend {
        StreamingPreviewBackend::Nemotron => nemotron_model_ready_in_dir(&model_path),
        StreamingPreviewBackend::Eou => eou_model_ready_in_dir(&model_path),
    };
    ready.then_some(StreamingPreviewConfig {
        backend,
        model_path,
        preferred_provider,
    })
}

pub(crate) struct StreamingPreviewEngine {
    backend: StreamingPreviewBackend,
    runtime: StreamingPreviewRuntime,
    pending_audio: Vec<f32>,
    transcript: String,
}

enum StreamingPreviewRuntime {
    Nemotron(Nemotron),
    Eou(ParakeetEOU),
}

impl StreamingPreviewEngine {
    pub(crate) fn load(config: &StreamingPreviewConfig) -> Result<Self> {
        let runtime = match config.backend {
            StreamingPreviewBackend::Nemotron => StreamingPreviewRuntime::Nemotron(
                load_nemotron_runtime(&config.model_path, config.preferred_provider)?,
            ),
            StreamingPreviewBackend::Eou => StreamingPreviewRuntime::Eou(load_eou_runtime(
                &config.model_path,
                config.preferred_provider,
            )?),
        };

        Ok(Self {
            backend: config.backend,
            runtime,
            pending_audio: Vec::new(),
            transcript: String::new(),
        })
    }

    pub(crate) fn diagnostic_backend(&self) -> &'static str {
        self.backend.diagnostic_backend()
    }

    pub(crate) fn active_detail(&self) -> &'static str {
        self.backend.active_detail()
    }

    pub(crate) fn push_audio(&mut self, audio_16khz: &[f32]) -> Result<Option<String>> {
        if audio_16khz.is_empty() {
            return Ok(None);
        }

        self.pending_audio.extend_from_slice(audio_16khz);
        let chunk_size = self.backend.chunk_size();
        let mut changed = false;

        while self.pending_audio.len() >= chunk_size {
            let chunk = self.pending_audio[..chunk_size].to_vec();
            self.pending_audio.drain(..chunk_size);

            match &mut self.runtime {
                StreamingPreviewRuntime::Nemotron(model) => {
                    model
                        .transcribe_chunk(&chunk)
                        .map_err(|error| anyhow!("Nemotron streaming inference failed: {error}"))?;
                    let transcript = model.get_transcript();
                    if !transcript.trim().is_empty() && transcript != self.transcript {
                        self.transcript = transcript;
                        changed = true;
                    }
                }
                StreamingPreviewRuntime::Eou(model) => {
                    let delta = model
                        .transcribe(&chunk, false)
                        .map_err(|error| anyhow!("Parakeet EOU inference failed: {error}"))?;
                    if !delta.trim().is_empty() {
                        self.transcript.push_str(&delta);
                        changed = true;
                    }
                }
            }
        }

        if changed {
            Ok(Some(self.transcript.clone()))
        } else {
            Ok(None)
        }
    }
}

fn load_nemotron_runtime(
    model_path: &Path,
    preferred_provider: crate::state::InferenceProvider,
) -> Result<Nemotron> {
    runtime::ensure_ort_initialized()?;
    load_with_provider_fallback(preferred_provider, |provider| {
        Nemotron::from_pretrained(model_path, Some(inference::execution_config(provider)))
            .map_err(|error| anyhow!("failed to load Nemotron streaming runtime: {error}"))
    })
}

fn load_eou_runtime(
    model_path: &Path,
    preferred_provider: crate::state::InferenceProvider,
) -> Result<ParakeetEOU> {
    runtime::ensure_ort_initialized()?;
    load_with_provider_fallback(preferred_provider, |provider| {
        ParakeetEOU::from_pretrained(model_path, Some(inference::execution_config(provider)))
            .map_err(|error| anyhow!("failed to load Parakeet EOU runtime: {error}"))
    })
}

fn load_with_provider_fallback<T>(
    preferred_provider: crate::state::InferenceProvider,
    loader: impl Fn(crate::state::InferenceProvider) -> Result<T>,
) -> Result<T> {
    match loader(preferred_provider) {
        Ok(runtime) => Ok(runtime),
        Err(primary_error) if preferred_provider != crate::state::InferenceProvider::Cpu => {
            loader(crate::state::InferenceProvider::Cpu)
                .with_context(|| format!("{primary_error}; falling back to CPU"))
        }
        Err(error) => Err(error),
    }
}

fn eou_model_ready_in_dir(model_dir: &Path) -> bool {
    EOU_REQUIRED_FILES
        .iter()
        .all(|filename| model_dir.join(filename).exists())
}

fn nemotron_model_ready_in_dir(model_dir: &Path) -> bool {
    NEMOTRON_REQUIRED_FILES
        .iter()
        .all(|filename| model_dir.join(filename).exists())
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::load_with_provider_fallback;
    use crate::state::InferenceProvider;

    #[test]
    fn load_with_provider_fallback_uses_cpu_after_webgpu_failure() {
        let value =
            load_with_provider_fallback(InferenceProvider::Webgpu, |provider| match provider {
                InferenceProvider::Webgpu => Err(anyhow!("webgpu failed")),
                InferenceProvider::Cpu => Ok("cpu"),
                InferenceProvider::Directml => unreachable!(),
            })
            .expect("cpu fallback should succeed");

        assert_eq!(value, "cpu");
    }

    #[test]
    fn load_with_provider_fallback_returns_cpu_error_without_retry() {
        let error = load_with_provider_fallback(InferenceProvider::Cpu, |_| {
            Err::<(), _>(anyhow!("cpu failed"))
        })
        .expect_err("cpu-only load should fail immediately");

        assert!(error.to_string().contains("cpu failed"));
    }
}
