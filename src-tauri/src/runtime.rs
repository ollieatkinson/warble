#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::{anyhow, Result};
use chrono::Utc;
use ort::logging::{LogLevel, LoggerFunction};
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use tauri::AppHandle;

use crate::storage::capture_log_path;

static ORT_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
static ORT_DIAGNOSTICS_CONFIG: OnceLock<OrtDiagnosticsConfig> = OnceLock::new();
static ORT_LOGGER: OnceLock<LoggerFunction> = OnceLock::new();
static ORT_LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone)]
struct OrtDiagnosticsConfig {
    log_path: Option<PathBuf>,
    log_level: LogLevel,
    log_verbosity: i32,
    echo_to_stderr: bool,
}

pub(crate) fn ensure_ort_initialized() -> Result<()> {
    let result =
        ORT_INIT_RESULT.get_or_init(|| initialize_ort_runtime().map_err(|error| error.to_string()));
    result.clone().map_err(|error| anyhow!(error))
}

pub(crate) fn configure_ort_diagnostics(app: &AppHandle) {
    let _ = ORT_DIAGNOSTICS_CONFIG.set(OrtDiagnosticsConfig {
        log_path: std::env::var_os("WARBLE_ORT_LOG_PATH")
            .map(PathBuf::from)
            .or_else(|| capture_log_path(app).ok()),
        log_level: ort_log_level(),
        log_verbosity: ort_log_verbosity(),
        echo_to_stderr: env_flag("WARBLE_ECHO_ORT_LOGS"),
    });

    append_runtime_diagnostic(
        "ONNX Runtime diagnostics configured",
        format!(
            "log_path={} log_level={} log_verbosity={} echo_to_stderr={} env_ort_log={} env_verbose={} env_timeout_ms={} env_graph_opt={} env_parallel_execution={} env_memory_pattern={}",
            ort_diagnostics_log_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            format_log_level(ort_log_level()),
            ort_log_verbosity(),
            env_flag("WARBLE_ECHO_ORT_LOGS"),
            std::env::var("ORT_LOG").unwrap_or_else(|_| "unset".to_string()),
            env_flag("WARBLE_ENABLE_ORT_VERBOSE_LOGS"),
            std::env::var("WARBLE_MACOS_WEBGPU_LOAD_TIMEOUT_MS")
                .unwrap_or_else(|_| "unset".to_string()),
            std::env::var("WARBLE_ORT_GRAPH_OPT_LEVEL")
                .unwrap_or_else(|_| "unset".to_string()),
            std::env::var("WARBLE_ORT_PARALLEL_EXECUTION")
                .unwrap_or_else(|_| "unset".to_string()),
            std::env::var("WARBLE_ORT_MEMORY_PATTERN")
                .unwrap_or_else(|_| "unset".to_string()),
        ),
    );
}

pub(crate) fn ort_logger() -> LoggerFunction {
    ORT_LOGGER
        .get_or_init(|| {
            std::sync::Arc::new(
                |level: LogLevel, category: &str, id: &str, code_location: &str, message: &str| {
                    append_runtime_diagnostic(
                        "ONNX Runtime",
                        format!(
                            "level={} category={} id={} code_location={} message={}",
                            format_log_level(level),
                            sanitize_log_field(category),
                            sanitize_log_field(id),
                            sanitize_log_field(code_location),
                            sanitize_log_field(message)
                        ),
                    );
                },
            )
        })
        .clone()
}

pub(crate) fn ort_log_level() -> LogLevel {
    if let Some(config) = ORT_DIAGNOSTICS_CONFIG.get() {
        return config.log_level;
    }

    default_ort_log_level()
}

pub(crate) fn ort_log_verbosity() -> i32 {
    if let Some(config) = ORT_DIAGNOSTICS_CONFIG.get() {
        return config.log_verbosity;
    }

    default_ort_log_verbosity(default_ort_log_level())
}

pub(crate) fn ort_diagnostics_log_path() -> Option<PathBuf> {
    ort_diagnostics_config().log_path.clone()
}

pub(crate) fn append_runtime_diagnostic(stage: &str, detail: impl Into<String>) {
    let detail = detail.into();
    let message = if detail.is_empty() {
        format!("{} [runtime] {stage}", Utc::now().to_rfc3339())
    } else {
        format!("{} [runtime] {stage}: {detail}", Utc::now().to_rfc3339())
    };

    let config = ort_diagnostics_config();
    if let Some(path) = config.log_path.as_ref() {
        let _lock = ORT_LOG_WRITE_LOCK.lock();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{message}");
        }
    }

    if config.echo_to_stderr {
        eprintln!("{message}");
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn directml_runtime_available() -> bool {
    let Some(runtime_dir) = ort_runtime_dir() else {
        return false;
    };

    [
        "onnxruntime.dll",
        "DirectML.dll",
        "dxcompiler.dll",
        "dxil.dll",
    ]
    .iter()
    .all(|file_name| runtime_dir.join(file_name).exists())
}

#[cfg(target_os = "windows")]
fn initialize_ort_runtime() -> Result<()> {
    let dylib_path = ort_dylib_path()
        .ok_or_else(|| anyhow!("onnxruntime.dll was not found next to the application"))?;
    append_runtime_diagnostic(
        "ONNX Runtime initialization started",
        format!(
            "mode=dynamic log_level={} log_verbosity={} dylib_path={} thread={:?}",
            format_log_level(ort_log_level()),
            ort_log_verbosity(),
            dylib_path.display(),
            thread::current().id()
        ),
    );
    let result = ort::init_from(&dylib_path)
        .with_context(|| {
            format!(
                "failed to prepare ONNX Runtime from {}",
                dylib_path.display()
            )
        })
        .map(|builder| builder.with_logger(ort_logger()));
    match result {
        Ok(builder) => {
            builder.commit();
            append_runtime_diagnostic(
                "ONNX Runtime initialization finished",
                format!(
                    "mode=dynamic log_level={} log_verbosity={} dylib_path={}",
                    format_log_level(ort_log_level()),
                    ort_log_verbosity(),
                    dylib_path.display()
                ),
            );
            Ok(())
        }
        Err(error) => {
            append_runtime_diagnostic(
                "ONNX Runtime initialization failed",
                format!(
                    "mode=dynamic dylib_path={} error={error}",
                    dylib_path.display()
                ),
            );
            Err(error)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn initialize_ort_runtime() -> Result<()> {
    append_runtime_diagnostic(
        "ONNX Runtime initialization started",
        format!(
            "mode=default log_level={} log_verbosity={} thread={:?}",
            format_log_level(ort_log_level()),
            ort_log_verbosity(),
            thread::current().id()
        ),
    );
    let _ = ort::init().with_logger(ort_logger()).commit();
    append_runtime_diagnostic(
        "ONNX Runtime initialization finished",
        format!(
            "mode=default log_level={} log_verbosity={}",
            format_log_level(ort_log_level()),
            ort_log_verbosity()
        ),
    );
    Ok(())
}

#[cfg(target_os = "windows")]
fn ort_dylib_path() -> Option<PathBuf> {
    std::env::var_os("ORT_DYLIB_PATH")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| {
            ort_runtime_dir()
                .map(|dir| dir.join("onnxruntime.dll"))
                .filter(|path| path.exists())
        })
}

#[cfg(target_os = "windows")]
fn ort_runtime_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
}

fn ort_diagnostics_config() -> &'static OrtDiagnosticsConfig {
    ORT_DIAGNOSTICS_CONFIG.get_or_init(|| OrtDiagnosticsConfig {
        log_path: std::env::var_os("WARBLE_ORT_LOG_PATH").map(PathBuf::from),
        log_level: default_ort_log_level(),
        log_verbosity: default_ort_log_verbosity(default_ort_log_level()),
        echo_to_stderr: env_flag("WARBLE_ECHO_ORT_LOGS"),
    })
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name),
        Ok(value) if matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    )
}

fn parse_log_level(value: &str) -> Option<LogLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "verbose" | "trace" => Some(LogLevel::Verbose),
        "info" => Some(LogLevel::Info),
        "warning" | "warn" => Some(LogLevel::Warning),
        "error" => Some(LogLevel::Error),
        "fatal" => Some(LogLevel::Fatal),
        _ => None,
    }
}

fn format_log_level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Verbose => "verbose",
        LogLevel::Info => "info",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
        LogLevel::Fatal => "fatal",
    }
}

fn sanitize_log_field(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn default_ort_log_level() -> LogLevel {
    if let Ok(value) = std::env::var("WARBLE_ORT_LOG_LEVEL") {
        if let Some(level) = parse_log_level(&value) {
            return level;
        }
    }

    if env_flag("WARBLE_ENABLE_ORT_VERBOSE_LOGS") {
        return LogLevel::Verbose;
    }

    std::env::var("ORT_LOG")
        .ok()
        .and_then(|value| parse_log_level(&value))
        .unwrap_or(LogLevel::Error)
}

fn default_ort_log_verbosity(level: LogLevel) -> i32 {
    std::env::var("WARBLE_ORT_LOG_VERBOSITY")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| {
            if matches!(level, LogLevel::Verbose) {
                1
            } else {
                0
            }
        })
}
