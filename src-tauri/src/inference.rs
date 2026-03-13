use anyhow::{anyhow, Context, Result};
use parakeet_rs::{ExecutionConfig, ExecutionProvider as LibraryExecutionProvider};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crate::platform::PlatformKind;
#[cfg(target_os = "windows")]
use crate::runtime;
use crate::state::{InferenceProvider, Settings};

#[cfg(target_os = "macos")]
const MACOS_WEBGPU_LOAD_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) const MACOS_RUNTIME_MODEL_IDS: &[&str] = &[
    "parakeet",
    "parakeet-ctc",
    "parakeet-eou",
    "nemotron-streaming",
];

#[derive(Debug, Clone)]
pub(crate) enum ProviderLoadEvent {
    AttemptStarted {
        provider: InferenceProvider,
        timeout: Option<Duration>,
    },
    AttemptFinished {
        provider: InferenceProvider,
        elapsed: Duration,
    },
    AttemptFailed {
        provider: InferenceProvider,
        elapsed: Duration,
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionConfigProfile {
    pub(crate) intra_threads: usize,
    pub(crate) inter_threads: usize,
    pub(crate) custom_configure: &'static str,
}

pub(crate) fn supported_acceleration_providers() -> Vec<InferenceProvider> {
    #[cfg(target_os = "windows")]
    let platform = PlatformKind::Windows;
    #[cfg(target_os = "macos")]
    let platform = PlatformKind::Macos;
    #[cfg(target_os = "linux")]
    let platform = PlatformKind::Linux;
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let platform = PlatformKind::Linux;

    #[cfg(target_os = "windows")]
    let directml_available = runtime::directml_runtime_available();
    #[cfg(not(target_os = "windows"))]
    let directml_available = false;

    supported_acceleration_providers_for_platform(platform, directml_available)
}

pub(crate) fn preferred_inference_providers() -> Vec<InferenceProvider> {
    #[cfg(target_os = "windows")]
    let platform = PlatformKind::Windows;
    #[cfg(target_os = "macos")]
    let platform = PlatformKind::Macos;
    #[cfg(target_os = "linux")]
    let platform = PlatformKind::Linux;
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let platform = PlatformKind::Linux;

    preferred_inference_providers_for_platform(platform, &supported_acceleration_providers())
}

pub(crate) fn is_macos_runtime_model(model_id: &str) -> bool {
    MACOS_RUNTIME_MODEL_IDS.contains(&model_id)
}

pub(crate) fn uses_explicit_provider_selection(platform: PlatformKind, model_id: &str) -> bool {
    matches!(platform, PlatformKind::Macos) && is_macos_runtime_model(model_id)
}

pub(crate) fn selected_provider_for_model(
    platform: PlatformKind,
    settings: &Settings,
    supported_providers: &[InferenceProvider],
    model_id: &str,
) -> InferenceProvider {
    if uses_explicit_provider_selection(platform, model_id) {
        return settings.macos_runtime_preference_for_model(model_id).into();
    }

    supported_providers
        .first()
        .copied()
        .unwrap_or(InferenceProvider::Cpu)
}

pub(crate) fn load_with_provider_fallback<T>(
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    let order = preferred_inference_providers();
    load_with_provider_order(&order, loader)
}

pub(crate) fn load_with_provider_fallback_and_observer<T>(
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
    observer: impl Fn(ProviderLoadEvent) + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    let order = preferred_inference_providers();
    load_with_provider_order_and_observer(&order, loader, observer)
}

pub(crate) fn load_with_exact_provider<T>(
    provider: InferenceProvider,
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    load_with_provider_order(&[provider], loader)
}

pub(crate) fn load_with_exact_provider_and_observer<T>(
    provider: InferenceProvider,
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
    observer: impl Fn(ProviderLoadEvent) + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    load_with_provider_order_and_observer(&[provider], loader, observer)
}

pub(crate) fn load_with_preferred_provider_then_cpu<T>(
    provider: InferenceProvider,
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    let order = if matches!(provider, InferenceProvider::Cpu) {
        vec![InferenceProvider::Cpu]
    } else {
        vec![provider, InferenceProvider::Cpu]
    };
    load_with_provider_order(&order, loader)
}

fn supported_acceleration_providers_for_platform(
    platform: PlatformKind,
    directml_available: bool,
) -> Vec<InferenceProvider> {
    match platform {
        PlatformKind::Windows if directml_available => vec![InferenceProvider::Directml],
        PlatformKind::Windows => Vec::new(),
        PlatformKind::Macos => vec![InferenceProvider::Coreml, InferenceProvider::Webgpu],
        PlatformKind::Linux => vec![InferenceProvider::Webgpu],
    }
}

fn preferred_inference_providers_for_platform(
    platform: PlatformKind,
    supported_providers: &[InferenceProvider],
) -> Vec<InferenceProvider> {
    let mut providers = match platform {
        PlatformKind::Macos => Vec::new(),
        PlatformKind::Windows | PlatformKind::Linux => supported_providers.to_vec(),
    };
    providers.push(InferenceProvider::Cpu);
    providers
}

fn load_with_provider_order<T>(
    order: &[InferenceProvider],
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    load_with_provider_order_and_observer(order, loader, |_| {})
}

fn load_with_provider_order_and_observer<T>(
    order: &[InferenceProvider],
    loader: impl Fn(InferenceProvider) -> Result<T> + Send + Sync + 'static,
    observer: impl Fn(ProviderLoadEvent) + Send + Sync + 'static,
) -> Result<(InferenceProvider, T)>
where
    T: Send + 'static,
{
    let mut acceleration_errors = Vec::new();
    let loader = Arc::new(loader);
    let observer = Arc::new(observer);

    for &provider in order {
        let timeout = provider_load_timeout(provider);
        observer(ProviderLoadEvent::AttemptStarted { provider, timeout });
        let started_at = Instant::now();
        let result = load_provider_with_optional_timeout(provider, timeout, {
            let loader = Arc::clone(&loader);
            move || loader(provider)
        });
        let elapsed = started_at.elapsed();

        match result {
            Ok(runtime) => {
                observer(ProviderLoadEvent::AttemptFinished { provider, elapsed });
                return Ok((provider, runtime));
            }
            Err(error) => {
                observer(ProviderLoadEvent::AttemptFailed {
                    provider,
                    elapsed,
                    error: error.to_string(),
                });
                if provider.is_accelerated() {
                    acceleration_errors.push(format!("{provider}: {error}"));
                    continue;
                }

                if acceleration_errors.is_empty() {
                    return Err(error);
                }

                let attempts = acceleration_errors.join("; ");
                return Err(error).with_context(|| format!("{attempts}; CPU fallback failed"));
            }
        }
    }

    if acceleration_errors.is_empty() {
        return Err(anyhow!("no inference providers were attempted"));
    }

    Err(anyhow!(acceleration_errors.join("; ")))
}

fn provider_load_timeout(provider: InferenceProvider) -> Option<Duration> {
    #[cfg(target_os = "macos")]
    if matches!(provider, InferenceProvider::Webgpu) {
        return Some(MACOS_WEBGPU_LOAD_TIMEOUT);
    }

    let _ = provider;
    None
}

fn load_provider_with_optional_timeout<T, F>(
    provider: InferenceProvider,
    timeout: Option<Duration>,
    loader: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let Some(timeout) = timeout else {
        return loader();
    };

    let (sender, receiver) = mpsc::channel();
    let provider_name = provider.to_string();
    thread::spawn(move || {
        let _ = sender.send(loader());
    });

    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(anyhow!(
            "{provider_name} model load timed out after {}s",
            timeout.as_secs()
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(anyhow!(
            "{provider_name} model load thread exited unexpectedly"
        )),
    }
}

pub(crate) fn execution_config(provider: InferenceProvider) -> ExecutionConfig {
    let profile = execution_config_profile(provider);
    let execution_provider = match provider {
        InferenceProvider::Cpu => LibraryExecutionProvider::Cpu,
        InferenceProvider::Coreml => {
            #[cfg(target_os = "macos")]
            {
                LibraryExecutionProvider::CoreML
            }
            #[cfg(not(target_os = "macos"))]
            {
                LibraryExecutionProvider::Cpu
            }
        }
        InferenceProvider::Directml => {
            #[cfg(target_os = "windows")]
            {
                LibraryExecutionProvider::DirectML
            }
            #[cfg(not(target_os = "windows"))]
            {
                LibraryExecutionProvider::Cpu
            }
        }
        InferenceProvider::Webgpu => {
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                LibraryExecutionProvider::WebGPU
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                LibraryExecutionProvider::Cpu
            }
        }
    };
    let needs_directml_tuning = matches!(provider, InferenceProvider::Directml);
    let config = ExecutionConfig::new()
        .with_execution_provider(execution_provider)
        .with_intra_threads(profile.intra_threads)
        .with_inter_threads(profile.inter_threads);

    if needs_directml_tuning {
        config.with_custom_configure(move |builder| {
            let builder = builder
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?;
            Ok(builder
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?)
        })
    } else {
        config
    }
}

pub(crate) fn execution_config_profile(provider: InferenceProvider) -> ExecutionConfigProfile {
    match provider {
        InferenceProvider::Directml => ExecutionConfigProfile {
            intra_threads: 4,
            inter_threads: 1,
            custom_configure:
                "graph_optimization=level1, parallel_execution=false, memory_pattern=false",
        },
        InferenceProvider::Cpu | InferenceProvider::Coreml | InferenceProvider::Webgpu => {
            ExecutionConfigProfile {
                intra_threads: 4,
                inter_threads: 1,
                custom_configure: "none",
            }
        }
    }
}

pub(crate) fn format_provider_list(providers: &[InferenceProvider]) -> String {
    if providers.is_empty() {
        return "none".to_string();
    }

    providers
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn provider_runtime_note(
    platform: PlatformKind,
    provider: InferenceProvider,
) -> Option<&'static str> {
    match (platform, provider) {
        (PlatformKind::Macos, InferenceProvider::Coreml) => Some(
            "parakeet-rs 0.3.4 does not include the issue #51 CoreML `.with_subgraphs(true)` patch and does not expose the MLProgram knob discussed in the ym2132 write-up, so Warble can only request the stock CoreML EP path.",
        ),
        (PlatformKind::Macos, InferenceProvider::Webgpu) => Some(
            "Warble keeps macOS WebGPU aligned with parakeet-rs defaults: WebGPU EP, intra_threads=4, inter_threads=1, no extra SessionBuilder override.",
        ),
        _ if provider.is_accelerated() => Some(
            "parakeet-rs registers CPU after the requested accelerator, so unsupported nodes may still execute on CPU inside ONNX Runtime.",
        ),
        _ => None,
    }
}

pub(crate) fn provider_failure_hint(
    platform: PlatformKind,
    provider: InferenceProvider,
) -> Option<&'static str> {
    match (platform, provider) {
        (PlatformKind::Macos, InferenceProvider::Coreml) => Some(
            "CoreML is still unstable for Parakeet on Apple. parakeet-rs issue #51 reports excessive graph partitioning, high unsupported-node counts, and repeated 'Context leak detected, CoreAnalytics returned false' warnings. The linked ym2132 write-up also calls out MLProgram as relevant, but parakeet-rs 0.3.4 neither exposes that setting nor includes the unmerged `.with_subgraphs(true)` patch from PR #51.",
        ),
        (PlatformKind::Macos, InferenceProvider::Webgpu) => Some(
            "WebGPU on Apple goes through ONNX Runtime's Dawn/Metal path. If this times out or hangs, capture the timeout, GPU model, and whether any provider-finished event appeared.",
        ),
        (PlatformKind::Linux, InferenceProvider::Webgpu) => Some(
            "WebGPU is still experimental for Parakeet. Capture the adapter/runtime error and whether CPU fallback succeeded.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::load_with_provider_order;
    use super::load_with_provider_order_and_observer;
    use super::ProviderLoadEvent;
    use crate::platform::PlatformKind;
    use crate::state::InferenceProvider;
    use crate::state::MacosModelRuntimePreference;
    use crate::state::Settings;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn load_with_provider_order_uses_cpu_after_acceleration_failure() {
        let order = [InferenceProvider::Webgpu, InferenceProvider::Cpu];
        let (provider, value) = load_with_provider_order(&order, |provider| match provider {
            InferenceProvider::Webgpu => Err(anyhow::anyhow!("webgpu failed")),
            InferenceProvider::Cpu => Ok("cpu"),
            InferenceProvider::Coreml | InferenceProvider::Directml => unreachable!(),
        })
        .expect("fallback should succeed");

        assert_eq!(provider, InferenceProvider::Cpu);
        assert_eq!(value, "cpu");
    }

    #[test]
    fn supported_acceleration_providers_for_windows_only_reports_directml_when_available() {
        assert_eq!(
            super::supported_acceleration_providers_for_platform(PlatformKind::Windows, true),
            vec![InferenceProvider::Directml]
        );
        assert!(
            super::supported_acceleration_providers_for_platform(PlatformKind::Windows, false)
                .is_empty()
        );
    }

    #[test]
    fn supported_acceleration_providers_match_platform_policy() {
        assert_eq!(
            super::supported_acceleration_providers_for_platform(PlatformKind::Macos, false),
            vec![InferenceProvider::Coreml, InferenceProvider::Webgpu]
        );
        assert_eq!(
            super::supported_acceleration_providers_for_platform(PlatformKind::Linux, false),
            vec![InferenceProvider::Webgpu]
        );
    }

    #[test]
    fn preferred_order_defaults_to_cpu_on_macos() {
        let order = super::preferred_inference_providers_for_platform(
            PlatformKind::Macos,
            &[InferenceProvider::Coreml],
        );
        assert_eq!(order, vec![InferenceProvider::Cpu]);
    }

    #[test]
    fn preferred_order_keeps_acceleration_before_cpu_on_linux() {
        let order = super::preferred_inference_providers_for_platform(
            PlatformKind::Linux,
            &[InferenceProvider::Webgpu],
        );
        assert_eq!(
            order,
            vec![InferenceProvider::Webgpu, InferenceProvider::Cpu]
        );
    }

    #[test]
    fn preferred_order_keeps_acceleration_before_cpu_on_windows() {
        let order = super::preferred_inference_providers_for_platform(
            PlatformKind::Windows,
            &[InferenceProvider::Directml],
        );
        assert_eq!(
            order,
            vec![InferenceProvider::Directml, InferenceProvider::Cpu]
        );
    }

    #[test]
    fn load_with_provider_order_first_success_skips_rest() {
        let order = [InferenceProvider::Directml, InferenceProvider::Cpu];
        let (provider, value) = load_with_provider_order(&order, |provider| match provider {
            InferenceProvider::Directml => Ok("directml-ok"),
            InferenceProvider::Cpu | InferenceProvider::Coreml | InferenceProvider::Webgpu => {
                panic!("should not be called")
            }
        })
        .expect("first provider should succeed");

        assert_eq!(provider, InferenceProvider::Directml);
        assert_eq!(value, "directml-ok");
    }

    #[test]
    fn load_with_provider_order_all_fail_returns_error() {
        let order = [InferenceProvider::Webgpu, InferenceProvider::Cpu];
        let result = load_with_provider_order(&order, |_| Err::<(), _>(anyhow::anyhow!("fail")));
        assert!(result.is_err());
    }

    #[test]
    fn load_with_preferred_provider_then_cpu_retries_cpu_after_directml_failure() {
        let (provider, value) =
            super::load_with_preferred_provider_then_cpu(InferenceProvider::Directml, |provider| {
                match provider {
                    InferenceProvider::Directml => Err(anyhow::anyhow!("directml failed")),
                    InferenceProvider::Cpu => Ok("cpu-ok"),
                    InferenceProvider::Coreml | InferenceProvider::Webgpu => unreachable!(),
                }
            })
            .expect("cpu fallback should succeed");

        assert_eq!(provider, InferenceProvider::Cpu);
        assert_eq!(value, "cpu-ok");
    }

    #[test]
    fn load_with_provider_order_single_cpu_success() {
        let order = [InferenceProvider::Cpu];
        let (provider, value) =
            load_with_provider_order(&order, |_| Ok("cpu-ok")).expect("should succeed");

        assert_eq!(provider, InferenceProvider::Cpu);
        assert_eq!(value, "cpu-ok");
    }

    #[test]
    fn load_provider_with_optional_timeout_times_out() {
        let result = super::load_provider_with_optional_timeout(
            InferenceProvider::Webgpu,
            Some(Duration::from_millis(10)),
            || {
                std::thread::sleep(Duration::from_millis(30));
                Ok::<_, anyhow::Error>("late")
            },
        );

        let error = result.expect_err("expected timeout");
        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    fn load_with_provider_order_reports_attempt_events() {
        let order = [InferenceProvider::Webgpu, InferenceProvider::Cpu];
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_for_observer = Arc::clone(&events);

        let (provider, value) = load_with_provider_order_and_observer(
            &order,
            |provider| match provider {
                InferenceProvider::Webgpu => Err(anyhow::anyhow!("webgpu failed")),
                InferenceProvider::Cpu => Ok("cpu-ok"),
                InferenceProvider::Coreml | InferenceProvider::Directml => unreachable!(),
            },
            move |event| {
                events_for_observer
                    .lock()
                    .expect("events lock poisoned")
                    .push(event);
            },
        )
        .expect("fallback should succeed");

        assert_eq!(provider, InferenceProvider::Cpu);
        assert_eq!(value, "cpu-ok");

        let events = events.lock().expect("events lock poisoned");
        assert!(matches!(
            events.as_slice(),
            [
                ProviderLoadEvent::AttemptStarted {
                    provider: InferenceProvider::Webgpu,
                    ..
                },
                ProviderLoadEvent::AttemptFailed {
                    provider: InferenceProvider::Webgpu,
                    ..
                },
                ProviderLoadEvent::AttemptStarted {
                    provider: InferenceProvider::Cpu,
                    ..
                },
                ProviderLoadEvent::AttemptFinished {
                    provider: InferenceProvider::Cpu,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn selected_provider_for_model_uses_macos_runtime_preferences_per_model() {
        let mut settings = Settings::default();
        settings
            .macos_model_runtime_preferences
            .insert("parakeet".to_string(), MacosModelRuntimePreference::Webgpu);

        assert_eq!(
            super::selected_provider_for_model(
                PlatformKind::Macos,
                &settings,
                &[InferenceProvider::Coreml, InferenceProvider::Webgpu],
                "parakeet",
            ),
            InferenceProvider::Webgpu
        );
        assert_eq!(
            super::selected_provider_for_model(
                PlatformKind::Macos,
                &settings,
                &[InferenceProvider::Coreml, InferenceProvider::Webgpu],
                "parakeet-ctc",
            ),
            InferenceProvider::Cpu
        );
    }

    #[test]
    fn selected_provider_for_model_keeps_system_default_off_macos() {
        let settings = Settings::default();

        assert_eq!(
            super::selected_provider_for_model(
                PlatformKind::Windows,
                &settings,
                &[InferenceProvider::Directml],
                "parakeet",
            ),
            InferenceProvider::Directml
        );
    }

    #[test]
    fn format_provider_list_formats_empty_and_multiple_lists() {
        assert_eq!(super::format_provider_list(&[]), "none");
        assert_eq!(
            super::format_provider_list(&[
                InferenceProvider::Coreml,
                InferenceProvider::Webgpu,
                InferenceProvider::Cpu,
            ]),
            "CoreML, WebGPU, CPU"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_webgpu_config_matches_upstream_defaults() {
        let config = super::execution_config(InferenceProvider::Webgpu);
        assert_eq!(config.intra_threads, 4);
        assert_eq!(config.inter_threads, 1);
        assert!(config.configure.is_none());
    }
}
