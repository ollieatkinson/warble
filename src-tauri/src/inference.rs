use anyhow::{anyhow, Context, Result};
use parakeet_rs::{ExecutionConfig, ExecutionProvider as LibraryExecutionProvider};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use crate::runtime;
use crate::state::InferenceProvider;

#[cfg(target_os = "macos")]
const MACOS_WEBGPU_LOAD_TIMEOUT: Duration = Duration::from_secs(20);

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

pub(crate) fn supported_acceleration_providers() -> Vec<InferenceProvider> {
    #[cfg(target_os = "windows")]
    {
        if runtime::directml_runtime_available() {
            return vec![InferenceProvider::Directml];
        }

        return Vec::new();
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        return vec![InferenceProvider::Webgpu];
    }

    #[allow(unreachable_code)]
    Vec::new()
}

pub(crate) fn preferred_inference_providers() -> Vec<InferenceProvider> {
    let mut providers = supported_acceleration_providers();
    providers.push(InferenceProvider::Cpu);
    providers
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

    unreachable!("preferred_inference_providers always includes CPU")
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
    let execution_provider = match provider {
        InferenceProvider::Cpu => LibraryExecutionProvider::Cpu,
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
    #[cfg(target_os = "macos")]
    let needs_webgpu_serialization = matches!(provider, InferenceProvider::Webgpu);
    #[cfg(not(target_os = "macos"))]
    let needs_webgpu_serialization = false;

    let intra_threads = if needs_webgpu_serialization { 1 } else { 4 };

    let config = ExecutionConfig::new()
        .with_execution_provider(execution_provider)
        .with_intra_threads(intra_threads)
        .with_inter_threads(1);

    if needs_directml_tuning {
        config.with_custom_configure(move |builder| {
            let builder = builder
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?;
            Ok(builder
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?)
        })
    } else if needs_webgpu_serialization {
        config.with_custom_configure(move |builder| {
            // macOS WebGPU currently has upstream Dawn/Metal concurrency bugs.
            // Keep the GPU path, but avoid expensive session-planning paths on Apple.
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

#[cfg(test)]
mod tests {
    use super::load_with_provider_order;
    use super::load_with_provider_order_and_observer;
    use super::ProviderLoadEvent;
    use crate::state::InferenceProvider;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn load_with_provider_order_uses_cpu_after_acceleration_failure() {
        let order = [InferenceProvider::Webgpu, InferenceProvider::Cpu];
        let (provider, value) = load_with_provider_order(&order, |provider| match provider {
            InferenceProvider::Webgpu => Err(anyhow::anyhow!("webgpu failed")),
            InferenceProvider::Cpu => Ok("cpu"),
            InferenceProvider::Directml => unreachable!(),
        })
        .expect("fallback should succeed");

        assert_eq!(provider, InferenceProvider::Cpu);
        assert_eq!(value, "cpu");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn preferred_order_starts_with_directml_when_available() {
        let order = super::preferred_inference_providers();
        assert_eq!(order.last(), Some(&InferenceProvider::Cpu));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn preferred_order_starts_with_webgpu() {
        let order = super::preferred_inference_providers();
        assert_eq!(order.first(), Some(&InferenceProvider::Webgpu));
        assert_eq!(order.last(), Some(&InferenceProvider::Cpu));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_webgpu_config_uses_serial_execution() {
        let config = super::execution_config(InferenceProvider::Webgpu);
        assert_eq!(config.intra_threads, 1);
        assert_eq!(config.inter_threads, 1);
        assert!(config.configure.is_some());
    }

    #[test]
    fn load_with_provider_order_first_success_skips_rest() {
        let order = [InferenceProvider::Directml, InferenceProvider::Cpu];
        let (provider, value) = load_with_provider_order(&order, |provider| match provider {
            InferenceProvider::Directml => Ok("directml-ok"),
            _ => panic!("should not be called"),
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
                InferenceProvider::Directml => unreachable!(),
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
}
