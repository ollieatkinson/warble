use anyhow::{Context, Result};
use parakeet_rs::{ExecutionConfig, ExecutionProvider as LibraryExecutionProvider};

#[cfg(target_os = "windows")]
use crate::runtime;
use crate::state::InferenceProvider;

pub(crate) fn supported_acceleration_providers() -> Vec<InferenceProvider> {
    #[cfg(target_os = "windows")]
    {
        if runtime::directml_runtime_available() {
            return vec![InferenceProvider::Directml];
        }

        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    {
        // WebGPU session creation is not reliable enough on macOS for the
        // default transcription path; prefer the stable CPU backend.
        return Vec::new();
    }

    #[cfg(target_os = "linux")]
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
    loader: impl Fn(InferenceProvider) -> Result<T>,
) -> Result<(InferenceProvider, T)> {
    let order = preferred_inference_providers();
    load_with_provider_order(&order, loader)
}

fn load_with_provider_order<T>(
    order: &[InferenceProvider],
    loader: impl Fn(InferenceProvider) -> Result<T>,
) -> Result<(InferenceProvider, T)> {
    let mut acceleration_errors = Vec::new();

    for provider in order {
        match loader(*provider) {
            Ok(runtime) => return Ok((*provider, runtime)),
            Err(error) => {
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

    let config = ExecutionConfig::new()
        .with_execution_provider(execution_provider)
        .with_intra_threads(4)
        .with_inter_threads(1);

    config.with_custom_configure(move |builder| {
        let builder = builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?;
        if needs_directml_tuning {
            Ok(builder
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?)
        } else {
            Ok(builder)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::load_with_provider_order;
    use crate::state::InferenceProvider;

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

    #[cfg(target_os = "linux")]
    #[test]
    fn preferred_order_starts_with_webgpu() {
        let order = super::preferred_inference_providers();
        assert_eq!(order.first(), Some(&InferenceProvider::Webgpu));
        assert_eq!(order.last(), Some(&InferenceProvider::Cpu));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preferred_order_is_cpu_only_on_macos() {
        let order = super::preferred_inference_providers();
        assert_eq!(order, vec![InferenceProvider::Cpu]);
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
}
