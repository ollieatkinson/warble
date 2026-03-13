use anyhow::{anyhow, Result};
use parakeet_rs::{Nemotron, ParakeetEOU};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::inference;
use crate::platform;
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
    selected_provider: crate::state::InferenceProvider,
}

impl StreamingPreviewConfig {
    pub(crate) fn diagnostic_backend(&self) -> &'static str {
        self.backend.diagnostic_backend()
    }

    pub(crate) fn trying_detail(&self) -> String {
        format!(
            "{} provider={} fallback_policy={} model_path={} {} note={}",
            self.backend.trying_detail(),
            self.selected_provider,
            if allow_streaming_provider_fallback(
                platform::current_platform(),
                self.selected_provider
            ) {
                "preferred-then-cpu"
            } else {
                "exact-provider"
            },
            self.model_path.display(),
            summarize_streaming_model_dir(self.backend, &self.model_path),
            inference::provider_runtime_note(platform::current_platform(), self.selected_provider)
                .unwrap_or("none")
        )
    }
}

pub(crate) fn resolve_streaming_preview_config(
    settings: &Settings,
    system_profile: &SystemProfile,
) -> Option<StreamingPreviewConfig> {
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
        if let Some(config) = ready_streaming_preview_config(settings, system_profile, backend) {
            return Some(config);
        }
    }

    None
}

fn ready_streaming_preview_config(
    settings: &Settings,
    system_profile: &SystemProfile,
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
        selected_provider: inference::selected_provider_for_model(
            platform::current_platform(),
            settings,
            &system_profile.supported_acceleration_providers,
            model_id,
        ),
    })
}

pub(crate) struct StreamingPreviewEngine {
    backend: StreamingPreviewBackend,
    selected_provider: crate::state::InferenceProvider,
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
                load_nemotron_runtime(&config.model_path, config.selected_provider)?,
            ),
            StreamingPreviewBackend::Eou => StreamingPreviewRuntime::Eou(load_eou_runtime(
                &config.model_path,
                config.selected_provider,
            )?),
        };

        Ok(Self {
            backend: config.backend,
            selected_provider: config.selected_provider,
            runtime,
            pending_audio: Vec::new(),
            transcript: String::new(),
        })
    }

    pub(crate) fn diagnostic_backend(&self) -> &'static str {
        self.backend.diagnostic_backend()
    }

    pub(crate) fn active_detail(&self) -> String {
        format!(
            "{} provider={}",
            self.backend.active_detail(),
            self.selected_provider
        )
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
    selected_provider: crate::state::InferenceProvider,
) -> Result<Nemotron> {
    runtime::ensure_ort_initialized()?;
    let model_path = model_path.to_path_buf();
    load_with_selected_provider(
        platform::current_platform(),
        selected_provider,
        move |provider| load_nemotron_runtime_with_diagnostics(&model_path, provider),
    )
}

fn load_eou_runtime(
    model_path: &Path,
    selected_provider: crate::state::InferenceProvider,
) -> Result<ParakeetEOU> {
    runtime::ensure_ort_initialized()?;
    let model_path = model_path.to_path_buf();
    load_with_selected_provider(
        platform::current_platform(),
        selected_provider,
        move |provider| load_eou_runtime_with_diagnostics(&model_path, provider),
    )
}

fn allow_streaming_provider_fallback(
    platform: platform::PlatformKind,
    provider: crate::state::InferenceProvider,
) -> bool {
    !matches!(platform, platform::PlatformKind::Macos)
        && provider != crate::state::InferenceProvider::Cpu
}

fn summarize_streaming_model_dir(backend: StreamingPreviewBackend, model_path: &Path) -> String {
    let required_files = match backend {
        StreamingPreviewBackend::Nemotron => NEMOTRON_REQUIRED_FILES,
        StreamingPreviewBackend::Eou => EOU_REQUIRED_FILES,
    };
    let missing_files = required_files
        .iter()
        .filter(|name| !model_path.join(name).exists())
        .copied()
        .collect::<Vec<_>>();
    let mut entries = std::fs::read_dir(model_path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    entries.sort();
    let entry_preview = if entries.is_empty() {
        "none".to_string()
    } else {
        entries
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "exists={} missing_files={} entry_count={} entries=[{}]",
        model_path.exists(),
        if missing_files.is_empty() {
            "none".to_string()
        } else {
            missing_files.join(", ")
        },
        entries.len(),
        entry_preview
    )
}

fn load_nemotron_runtime_with_diagnostics(
    model_path: &Path,
    provider: crate::state::InferenceProvider,
) -> Result<Nemotron> {
    let profile = inference::execution_config_profile(provider);
    runtime::append_runtime_diagnostic(
        "Nemotron streaming runtime load started",
        format!(
            "provider={} model_path={} summary={} intra_threads={} inter_threads={} custom_configure={} session_overrides={} note={}",
            provider,
            model_path.display(),
            summarize_streaming_model_dir(StreamingPreviewBackend::Nemotron, model_path),
            profile.intra_threads,
            profile.inter_threads,
            profile.custom_configure,
            inference::session_override_summary(provider),
            inference::provider_runtime_note(platform::current_platform(), provider).unwrap_or("none")
        ),
    );
    let started_at = Instant::now();
    let result = Nemotron::from_pretrained(model_path, Some(inference::execution_config(provider)))
        .map_err(|error| anyhow!("failed to load Nemotron streaming runtime: {error}"));
    runtime::append_runtime_diagnostic(
        "Nemotron streaming runtime load finished",
        format!(
            "provider={} model_path={} elapsed_ms={} result={}",
            provider,
            model_path.display(),
            started_at.elapsed().as_millis(),
            match &result {
                Ok(_) => "ok".to_string(),
                Err(error) => format!("error={error}"),
            }
        ),
    );
    result
}

fn load_eou_runtime_with_diagnostics(
    model_path: &Path,
    provider: crate::state::InferenceProvider,
) -> Result<ParakeetEOU> {
    let profile = inference::execution_config_profile(provider);
    runtime::append_runtime_diagnostic(
        "Parakeet EOU runtime load started",
        format!(
            "provider={} model_path={} summary={} intra_threads={} inter_threads={} custom_configure={} session_overrides={} note={}",
            provider,
            model_path.display(),
            summarize_streaming_model_dir(StreamingPreviewBackend::Eou, model_path),
            profile.intra_threads,
            profile.inter_threads,
            profile.custom_configure,
            inference::session_override_summary(provider),
            inference::provider_runtime_note(platform::current_platform(), provider).unwrap_or("none")
        ),
    );
    let started_at = Instant::now();
    let result =
        ParakeetEOU::from_pretrained(model_path, Some(inference::execution_config(provider)))
            .map_err(|error| anyhow!("failed to load Parakeet EOU runtime: {error}"));
    runtime::append_runtime_diagnostic(
        "Parakeet EOU runtime load finished",
        format!(
            "provider={} model_path={} elapsed_ms={} result={}",
            provider,
            model_path.display(),
            started_at.elapsed().as_millis(),
            match &result {
                Ok(_) => "ok".to_string(),
                Err(error) => format!("error={error}"),
            }
        ),
    );
    result
}

fn load_with_selected_provider<T>(
    platform: platform::PlatformKind,
    selected_provider: crate::state::InferenceProvider,
    loader: impl Fn(crate::state::InferenceProvider) -> Result<T> + Send + Sync + 'static,
) -> Result<T>
where
    T: Send + 'static,
{
    let (_, runtime) = if allow_streaming_provider_fallback(platform, selected_provider) {
        inference::load_with_preferred_provider_then_cpu(selected_provider, loader).map_err(
            |error| {
                anyhow!(
                    "selected_provider={} strategy=preferred-then-cpu error={} hint={}",
                    selected_provider,
                    error,
                    inference::provider_failure_hint(platform, selected_provider).unwrap_or("none")
                )
            },
        )?
    } else {
        inference::load_with_exact_provider(selected_provider, loader).map_err(|error| {
            anyhow!(
                "selected_provider={} strategy=exact-provider error={} hint={}",
                selected_provider,
                error,
                inference::provider_failure_hint(platform, selected_provider).unwrap_or("none")
            )
        })?
    };

    Ok(runtime)
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

    use super::allow_streaming_provider_fallback;
    use super::load_with_selected_provider;
    use crate::platform::PlatformKind;
    use crate::state::InferenceProvider;

    #[test]
    fn load_with_selected_provider_uses_cpu_on_linux() {
        let value = load_with_selected_provider(
            PlatformKind::Linux,
            InferenceProvider::Cpu,
            |provider| match provider {
                InferenceProvider::Cpu => Ok("cpu"),
                InferenceProvider::Coreml
                | InferenceProvider::Directml
                | InferenceProvider::Webgpu => unreachable!(),
            },
        )
        .expect("cpu load should succeed");

        assert_eq!(value, "cpu");
    }

    #[test]
    fn load_with_selected_provider_uses_cpu_after_directml_failure_on_windows() {
        let value = load_with_selected_provider(
            PlatformKind::Windows,
            InferenceProvider::Directml,
            |provider| match provider {
                InferenceProvider::Directml => Err(anyhow!("directml failed")),
                InferenceProvider::Cpu => Ok("cpu"),
                InferenceProvider::Coreml | InferenceProvider::Webgpu => unreachable!(),
            },
        )
        .expect("cpu fallback should succeed");

        assert_eq!(value, "cpu");
    }

    #[test]
    fn load_with_selected_provider_returns_cpu_error_without_retry() {
        let error =
            load_with_selected_provider(PlatformKind::Linux, InferenceProvider::Cpu, |_| {
                Err::<(), _>(anyhow!("cpu failed"))
            })
            .expect_err("cpu-only load should fail immediately");

        assert!(error.to_string().contains("cpu failed"));
    }

    #[test]
    fn load_with_selected_provider_does_not_fallback_when_macos_coreml_fails() {
        let error = load_with_selected_provider(
            PlatformKind::Macos,
            InferenceProvider::Coreml,
            |provider| match provider {
                InferenceProvider::Coreml => Err::<(), _>(anyhow!("coreml failed")),
                InferenceProvider::Cpu
                | InferenceProvider::Directml
                | InferenceProvider::Webgpu => unreachable!(),
            },
        )
        .expect_err("macOS explicit provider selection should fail immediately");

        assert!(error.to_string().contains("coreml failed"));
    }

    #[test]
    fn allow_streaming_provider_fallback_is_disabled_on_macos() {
        assert!(!allow_streaming_provider_fallback(
            PlatformKind::Macos,
            InferenceProvider::Coreml
        ));
        assert!(allow_streaming_provider_fallback(
            PlatformKind::Windows,
            InferenceProvider::Directml
        ));
        assert!(allow_streaming_provider_fallback(
            PlatformKind::Linux,
            InferenceProvider::Cpu
        ));
    }

    #[test]
    fn eou_model_ready_requires_all_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!super::eou_model_ready_in_dir(dir.path()));

        for file in super::EOU_REQUIRED_FILES {
            std::fs::write(dir.path().join(file), "").unwrap();
        }
        assert!(super::eou_model_ready_in_dir(dir.path()));
    }

    #[test]
    fn nemotron_model_ready_requires_all_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!super::nemotron_model_ready_in_dir(dir.path()));

        for file in super::NEMOTRON_REQUIRED_FILES {
            std::fs::write(dir.path().join(file), "").unwrap();
        }
        assert!(super::nemotron_model_ready_in_dir(dir.path()));
    }
}
