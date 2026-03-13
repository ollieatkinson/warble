use anyhow::{anyhow, Context, Result};
use parakeet_rs::{ExecutionConfig, ExecutionProvider as LibraryExecutionProvider};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crate::platform::PlatformKind;
use crate::runtime;
use crate::state::{InferenceProvider, Settings};

#[cfg(target_os = "macos")]
const MACOS_WEBGPU_LOAD_TIMEOUT: Duration = Duration::from_secs(20);
const PROVIDER_LOAD_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

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
        return Some(macos_webgpu_load_timeout());
    }

    let _ = provider;
    None
}

#[cfg(target_os = "macos")]
fn macos_webgpu_load_timeout() -> Duration {
    std::env::var("WARBLE_MACOS_WEBGPU_LOAD_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(MACOS_WEBGPU_LOAD_TIMEOUT)
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
        let started_at = Instant::now();
        runtime::append_runtime_diagnostic(
            "Inference provider load started",
            format!(
                "provider={} timeout_ms=none thread={:?}",
                provider,
                thread::current().id()
            ),
        );
        let result = loader();
        runtime::append_runtime_diagnostic(
            "Inference provider load finished",
            format!(
                "provider={} timeout_ms=none elapsed_ms={} result={}",
                provider,
                started_at.elapsed().as_millis(),
                match &result {
                    Ok(_) => "ok".to_string(),
                    Err(error) => format!("error={error}"),
                }
            ),
        );
        return result;
    };

    #[cfg(target_os = "macos")]
    if matches!(provider, InferenceProvider::Webgpu) {
        return load_provider_with_inline_watchdog(provider, timeout, loader);
    }

    let (sender, receiver) = mpsc::channel();
    let provider_name = provider.to_string();
    let provider_name_for_thread = provider_name.clone();
    runtime::append_runtime_diagnostic(
        "Inference provider load guard armed",
        format!(
            "provider={} timeout_ms={} heartbeat_ms={} caller_thread={:?}",
            provider,
            timeout.as_millis(),
            PROVIDER_LOAD_HEARTBEAT_INTERVAL.as_millis(),
            thread::current().id()
        ),
    );
    thread::spawn(move || {
        let started_at = Instant::now();
        runtime::append_runtime_diagnostic(
            "Inference provider background load started",
            format!(
                "provider={} worker_thread={:?}",
                provider_name_for_thread,
                thread::current().id()
            ),
        );
        let result = loader();
        let elapsed = started_at.elapsed();
        let outcome = match &result {
            Ok(_) => "ok".to_string(),
            Err(error) => format!("error={error}"),
        };
        let delivered = sender.send(result).is_ok();
        runtime::append_runtime_diagnostic(
            "Inference provider background load finished",
            format!(
                "provider={} elapsed_ms={} delivered_to_waiter={} {}",
                provider_name_for_thread,
                elapsed.as_millis(),
                delivered,
                outcome
            ),
        );
    });

    let started_at = Instant::now();
    loop {
        let elapsed = started_at.elapsed();
        let Some(remaining) = timeout.checked_sub(elapsed) else {
            runtime::append_runtime_diagnostic(
                "Inference provider load timed out",
                format!(
                    "provider={} elapsed_ms={} timeout_ms={}",
                    provider_name,
                    elapsed.as_millis(),
                    timeout.as_millis()
                ),
            );
            return Err(anyhow!(
                "{provider_name} model load timed out after {}s",
                timeout.as_secs()
            ));
        };
        let wait_for = remaining.min(PROVIDER_LOAD_HEARTBEAT_INTERVAL);

        match receiver.recv_timeout(wait_for) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) if started_at.elapsed() >= timeout => {
                let elapsed = started_at.elapsed();
                runtime::append_runtime_diagnostic(
                    "Inference provider load timed out",
                    format!(
                        "provider={} elapsed_ms={} timeout_ms={}",
                        provider_name,
                        elapsed.as_millis(),
                        timeout.as_millis()
                    ),
                );
                return Err(anyhow!(
                    "{provider_name} model load timed out after {}s",
                    timeout.as_secs()
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                runtime::append_runtime_diagnostic(
                    "Inference provider load heartbeat",
                    format!(
                        "provider={} elapsed_ms={} timeout_ms={} waiting_for_background_loader=true",
                        provider_name,
                        started_at.elapsed().as_millis(),
                        timeout.as_millis()
                    ),
                );
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                runtime::append_runtime_diagnostic(
                    "Inference provider load thread disconnected",
                    format!("provider={} before_result=true", provider_name),
                );
                return Err(anyhow!(
                    "{provider_name} model load thread exited unexpectedly"
                ));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn load_provider_with_inline_watchdog<T, F>(
    provider: InferenceProvider,
    timeout: Duration,
    loader: F,
) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let (stop_sender, stop_receiver) = mpsc::channel::<()>();
    let provider_name = provider.to_string();
    let provider_name_for_watchdog = provider_name.clone();
    let started_at = Instant::now();

    runtime::append_runtime_diagnostic(
        "Inference provider load inline watchdog armed",
        format!(
            "provider={} timeout_ms={} heartbeat_ms={} caller_thread={:?} reason=macos-webgpu-matches-upstream-threading",
            provider,
            timeout.as_millis(),
            PROVIDER_LOAD_HEARTBEAT_INTERVAL.as_millis(),
            thread::current().id()
        ),
    );

    let watchdog = thread::spawn(move || {
        let mut advisory_timeout_reported = false;
        loop {
            match stop_receiver.recv_timeout(PROVIDER_LOAD_HEARTBEAT_INTERVAL) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let elapsed = started_at.elapsed();
                    if elapsed >= timeout {
                        runtime::append_runtime_diagnostic(
                            if advisory_timeout_reported {
                                "Inference provider load still waiting after advisory timeout"
                            } else {
                                "Inference provider load exceeded advisory timeout"
                            },
                            format!(
                                "provider={} elapsed_ms={} timeout_ms={} waiting_for_inline_loader=true interruptible=false",
                                provider_name_for_watchdog,
                                elapsed.as_millis(),
                                timeout.as_millis()
                            ),
                        );
                        advisory_timeout_reported = true;
                    } else {
                        runtime::append_runtime_diagnostic(
                            "Inference provider load heartbeat",
                            format!(
                                "provider={} elapsed_ms={} timeout_ms={} waiting_for_inline_loader=true interruptible=false",
                                provider_name_for_watchdog,
                                elapsed.as_millis(),
                                timeout.as_millis()
                            ),
                        );
                    }
                }
            }
        }
    });

    runtime::append_runtime_diagnostic(
        "Inference provider load started",
        format!(
            "provider={} timeout_ms={} thread={:?} mode=inline-watchdog",
            provider,
            timeout.as_millis(),
            thread::current().id()
        ),
    );
    let result = loader();
    let elapsed = started_at.elapsed();
    let _ = stop_sender.send(());
    let _ = watchdog.join();
    runtime::append_runtime_diagnostic(
        "Inference provider load finished",
        format!(
            "provider={} timeout_ms={} elapsed_ms={} mode=inline-watchdog result={}",
            provider_name,
            timeout.as_millis(),
            elapsed.as_millis(),
            match &result {
                Ok(_) => "ok".to_string(),
                Err(error) => format!("error={error}"),
            }
        ),
    );

    result
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
    let config = ExecutionConfig::new()
        .with_execution_provider(execution_provider)
        .with_intra_threads(profile.intra_threads)
        .with_inter_threads(profile.inter_threads);

    if session_builder_override_requested(provider) {
        return config
            .with_custom_configure(move |builder| configure_session_builder(builder, provider));
    }

    config
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
            "Warble keeps macOS WebGPU aligned with parakeet-rs defaults for execution settings: WebGPU EP, intra_threads=4, inter_threads=1, no extra SessionBuilder hooks. The 20s threshold is only an advisory heartbeat on Apple because detached timeout threads can leave ONNX Runtime wedged after a partial WebGPU load.",
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
            "WebGPU on Apple goes through ONNX Runtime's Dawn/Metal path. Capture the GPU model, whether the advisory timeout was exceeded, and whether any provider-finished event appeared. GitHub-hosted macOS runners currently expose an Apple Virtual Machine GPU, so accelerator fixture results there are not representative of bare-metal Macs.",
        ),
        (PlatformKind::Linux, InferenceProvider::Webgpu) => Some(
            "WebGPU is still experimental for Parakeet. Capture the adapter/runtime error and whether CPU fallback succeeded.",
        ),
        _ => None,
    }
}

pub(crate) fn session_override_summary(provider: InferenceProvider) -> String {
    let mut overrides = Vec::new();
    if matches!(provider, InferenceProvider::Directml) {
        overrides.push(
            "graph_optimization=level1, parallel_execution=false, memory_pattern=false".to_string(),
        );
    }
    if let Some(level) = env_graph_optimization_level() {
        overrides.push(format!(
            "graph_optimization={}",
            graph_optimization_level_label(level)
        ));
    }
    if let Some(enabled) = env_parallel_execution_override() {
        overrides.push(format!("parallel_execution={enabled}"));
    }
    if let Some(enabled) = env_memory_pattern_override() {
        overrides.push(format!("memory_pattern={enabled}"));
    }

    if overrides.is_empty() {
        "none".to_string()
    } else {
        overrides.join(", ")
    }
}

fn session_builder_override_requested(provider: InferenceProvider) -> bool {
    matches!(provider, InferenceProvider::Directml)
        || env_graph_optimization_level().is_some()
        || env_parallel_execution_override().is_some()
        || env_memory_pattern_override().is_some()
}

fn configure_session_builder(
    builder: ort::session::builder::SessionBuilder,
    provider: InferenceProvider,
) -> ort::Result<ort::session::builder::SessionBuilder> {
    let mut builder = builder;
    if matches!(provider, InferenceProvider::Directml) {
        builder = builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?
            .with_parallel_execution(false)?
            .with_memory_pattern(false)?;
    }
    if let Some(level) = env_graph_optimization_level() {
        builder = builder.with_optimization_level(level)?;
    }
    if let Some(enabled) = env_parallel_execution_override() {
        builder = builder.with_parallel_execution(enabled)?;
    }
    if let Some(enabled) = env_memory_pattern_override() {
        builder = builder.with_memory_pattern(enabled)?;
    }

    Ok(builder)
}

fn env_graph_optimization_level() -> Option<ort::session::builder::GraphOptimizationLevel> {
    std::env::var("WARBLE_ORT_GRAPH_OPT_LEVEL")
        .ok()
        .and_then(|value| parse_graph_optimization_level(&value))
}

fn parse_graph_optimization_level(
    value: &str,
) -> Option<ort::session::builder::GraphOptimizationLevel> {
    use ort::session::builder::GraphOptimizationLevel;

    match value.trim().to_ascii_lowercase().as_str() {
        "disable" | "disabled" | "0" | "off" | "false" => Some(GraphOptimizationLevel::Disable),
        "level1" | "basic" | "1" => Some(GraphOptimizationLevel::Level1),
        "level2" | "extended" | "2" => Some(GraphOptimizationLevel::Level2),
        "level3" | "layout" | "3" => Some(GraphOptimizationLevel::Level3),
        "all" | "enable_all" | "4" => Some(GraphOptimizationLevel::All),
        _ => None,
    }
}

fn graph_optimization_level_label(
    level: ort::session::builder::GraphOptimizationLevel,
) -> &'static str {
    use ort::session::builder::GraphOptimizationLevel;

    match level {
        GraphOptimizationLevel::Disable => "disable",
        GraphOptimizationLevel::Level1 => "level1",
        GraphOptimizationLevel::Level2 => "level2",
        GraphOptimizationLevel::Level3 => "level3",
        GraphOptimizationLevel::All => "all",
    }
}

fn env_parallel_execution_override() -> Option<bool> {
    std::env::var("WARBLE_ORT_PARALLEL_EXECUTION")
        .ok()
        .and_then(|value| parse_env_bool_override(&value))
}

fn env_memory_pattern_override() -> Option<bool> {
    std::env::var("WARBLE_ORT_MEMORY_PATTERN")
        .ok()
        .and_then(|value| parse_env_bool_override(&value))
}

fn parse_env_bool_override(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
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
            InferenceProvider::Cpu,
            Some(Duration::from_millis(10)),
            || {
                std::thread::sleep(Duration::from_millis(250));
                Ok::<_, anyhow::Error>("late")
            },
        );

        let error = result.expect_err("expected timeout");
        assert!(error.to_string().contains("timed out"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn load_provider_with_optional_timeout_keeps_macos_webgpu_inline() {
        let result = super::load_provider_with_optional_timeout(
            InferenceProvider::Webgpu,
            Some(Duration::from_millis(5)),
            || {
                std::thread::sleep(Duration::from_millis(25));
                Ok::<_, anyhow::Error>("ok")
            },
        )
        .expect("macOS WebGPU uses inline watchdog instead of timing out");

        assert_eq!(result, "ok");
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
        let profile = super::execution_config_profile(InferenceProvider::Webgpu);
        assert_eq!(config.intra_threads, 4);
        assert_eq!(config.inter_threads, 1);
        assert_eq!(profile.custom_configure, "none");
        assert!(config.configure.is_none());
    }
}
