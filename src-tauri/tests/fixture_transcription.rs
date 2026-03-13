// Fixture-based transcription tests.
//
// The helper utilities (WAV generation, scoring) are tested directly.
// Tests that require a loaded model are gated with `#[ignore]`.

use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::time::Instant;

/// Generate a sine wave at the given frequency.
fn generate_sine(freq: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    (0..num_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
        .collect()
}

/// Generate silence (all zeros).
fn generate_silence(duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    vec![0.0f32; (duration_secs * sample_rate as f32) as usize]
}

/// Write mono PCM16 WAV to disk.
fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create WAV");
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        writer
            .write_sample((clamped * i16::MAX as f32) as i16)
            .expect("write sample");
    }
    writer.finalize().expect("finalize WAV");
}

/// Score how many expected words appear in the actual transcription.
fn word_match_score(expected: &[&str], actual: &str) -> f64 {
    if expected.is_empty() {
        return if actual.trim().is_empty() { 1.0 } else { 0.0 };
    }
    let actual_lower = actual.to_lowercase();
    let actual_words: Vec<&str> = actual_lower.split_whitespace().collect();
    let matched = expected
        .iter()
        .filter(|w| {
            let lower = w.to_lowercase();
            actual_words.iter().any(|a| *a == lower.as_str())
        })
        .count();
    matched as f64 / expected.len() as f64
}

fn repo_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("vosk-test.wav")
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug)]
enum MacosFixtureRuntime {
    Cpu,
    Coreml,
    Webgpu,
}

#[cfg(target_os = "macos")]
impl MacosFixtureRuntime {
    fn label(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Coreml => "coreml",
            Self::Webgpu => "webgpu",
        }
    }
}

#[cfg(target_os = "macos")]
fn coreml_fixture_enabled() -> bool {
    matches!(
        std::env::var("WARBLE_ENABLE_COREML_FIXTURE"),
        Ok(value) if matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    )
}

#[cfg(target_os = "macos")]
fn ort_log_path() -> Option<PathBuf> {
    std::env::var("WARBLE_ORT_LOG_PATH").ok().map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn print_ort_log_tail(context: &str) {
    let Some(path) = ort_log_path() else {
        eprintln!(
            "fixture ort log unavailable: context={context} reason=WARBLE_ORT_LOG_PATH unset"
        );
        return;
    };

    let Ok(contents) = std::fs::read_to_string(&path) else {
        eprintln!(
            "fixture ort log unavailable: context={context} path={} reason=read-failed",
            path.display()
        );
        return;
    };

    let tail = contents
        .lines()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    eprintln!(
        "fixture ort log tail start: context={context} path={}\n{}\nfixture ort log tail end",
        path.display(),
        tail
    );
}

#[cfg(target_os = "macos")]
fn run_macos_ci_fixture(runtime: MacosFixtureRuntime) {
    let wav_path = PathBuf::from(
        std::env::var("WARBLE_FIXTURE_WAV")
            .expect("set WARBLE_FIXTURE_WAV to a deterministic CI speech fixture"),
    );
    let expected_words = std::env::var("WARBLE_EXPECTED_WORDS")
        .expect("set WARBLE_EXPECTED_WORDS to the expected phrase");
    let expected = expected_words.split_whitespace().collect::<Vec<_>>();
    let model_root =
        PathBuf::from(std::env::var("WARBLE_MODEL_ROOT").expect("set WARBLE_MODEL_ROOT"));

    eprintln!(
        "fixture start: runtime={} model_root={} wav_path={} expected_words={expected_words:?} ort_log_path={} ort_verbose={} ort_echo={} webgpu_timeout_ms={}",
        runtime.label(),
        model_root.display(),
        wav_path.display(),
        ort_log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unset".to_string()),
        std::env::var("WARBLE_ENABLE_ORT_VERBOSE_LOGS").unwrap_or_else(|_| "unset".to_string()),
        std::env::var("WARBLE_ECHO_ORT_LOGS").unwrap_or_else(|_| "unset".to_string()),
        std::env::var("WARBLE_MACOS_WEBGPU_LOAD_TIMEOUT_MS")
            .unwrap_or_else(|_| "unset".to_string())
    );

    let load_started = Instant::now();
    let mut model = match runtime {
        MacosFixtureRuntime::Cpu => {
            match warble_lib::parakeet::ParakeetTdt::load_with_cpu(&model_root) {
                Ok(model) => model,
                Err(error) => {
                    print_ort_log_tail("load-cpu-failed");
                    panic!("load cpu fixture model: {error}");
                }
            }
        }
        MacosFixtureRuntime::Coreml => {
            match warble_lib::parakeet::ParakeetTdt::load_with_coreml(&model_root) {
                Ok(model) => model,
                Err(error) => {
                    print_ort_log_tail("load-coreml-failed");
                    panic!("load coreml fixture model: {error}");
                }
            }
        }
        MacosFixtureRuntime::Webgpu => {
            match warble_lib::parakeet::ParakeetTdt::load_with_webgpu(&model_root) {
                Ok(model) => model,
                Err(error) => {
                    print_ort_log_tail("load-webgpu-failed");
                    panic!("load webgpu fixture model: {error}");
                }
            }
        }
    };
    eprintln!(
        "fixture load finished: runtime={} elapsed_ms={}",
        runtime.label(),
        load_started.elapsed().as_millis()
    );

    let transcription_started = Instant::now();
    let result = match model.transcribe_wav_path(&wav_path) {
        Ok(result) => result,
        Err(error) => {
            print_ort_log_tail("transcribe-failed");
            panic!("transcribe macOS CI fixture: {error}");
        }
    };
    let score = word_match_score(&expected, &result);

    eprintln!(
        "fixture transcription finished: runtime={} elapsed_ms={} score={score:.2} actual={result:?}",
        runtime.label(),
        transcription_started.elapsed().as_millis()
    );

    assert!(
        score >= 0.75,
        "Fixture transcript mismatch: runtime={} score={score:.2} expected={expected_words:?} actual={result:?}",
        runtime.label()
    );
}

// ---------------------------------------------------------------------------
// WAV round-trip
// ---------------------------------------------------------------------------

#[test]
fn wav_write_read_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.wav");
    let samples = generate_sine(440.0, 1.0, 16000);
    write_wav(&path, &samples, 16000);

    let reader = hound::WavReader::open(&path).expect("open WAV");
    assert_eq!(reader.spec().sample_rate, 16000);
    assert_eq!(reader.spec().channels, 1);
    assert_eq!(reader.len() as usize, samples.len());
}

// ---------------------------------------------------------------------------
// Fixture generators
// ---------------------------------------------------------------------------

#[test]
fn silence_fixture_is_all_zeros() {
    let samples = generate_silence(1.0, 16000);
    assert_eq!(samples.len(), 16000);
    assert!(samples.iter().all(|&s| s == 0.0));
}

#[test]
fn sine_fixture_has_expected_properties() {
    let samples = generate_sine(440.0, 1.0, 16000);
    assert_eq!(samples.len(), 16000);
    // RMS should be ~0.707 for a unit sine wave
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!((rms - 0.707).abs() < 0.01, "RMS was {rms}, expected ~0.707");
}

#[test]
fn committed_speech_fixture_has_expected_format() {
    let path = repo_fixture_path();
    let reader = hound::WavReader::open(&path).expect("open committed fixture");
    let spec = reader.spec();

    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(spec.bits_per_sample, 16);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    assert!(
        reader.duration() > 100_000,
        "fixture should contain real speech"
    );
}

// ---------------------------------------------------------------------------
// Word match scoring
// ---------------------------------------------------------------------------

#[test]
fn word_match_score_exact() {
    assert!((word_match_score(&["hello", "world"], "Hello World") - 1.0).abs() < f64::EPSILON);
}

#[test]
fn word_match_score_partial() {
    assert!((word_match_score(&["hello", "world"], "hello there") - 0.5).abs() < f64::EPSILON);
}

#[test]
fn word_match_score_empty_expected_empty_actual() {
    assert!((word_match_score(&[], "") - 1.0).abs() < f64::EPSILON);
}

#[test]
fn word_match_score_empty_expected_nonempty_actual() {
    assert!((word_match_score(&[], "hello") - 0.0).abs() < f64::EPSILON);
}

#[test]
fn word_match_score_no_matches() {
    assert!((word_match_score(&["apple", "banana"], "cherry date") - 0.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Model-dependent transcription tests (require a downloaded model)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn transcribe_silence_returns_empty_or_short() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("silence.wav");
    let samples = generate_silence(2.0, 16000);
    write_wav(&path, &samples, 16000);

    // This test requires a loaded ParakeetTdt model.
    // It validates that silence does not produce spurious transcription.
    let model_root = std::path::PathBuf::from(
        std::env::var("WARBLE_MODEL_ROOT").expect("set WARBLE_MODEL_ROOT"),
    );
    let mut model = warble_lib::parakeet::ParakeetTdt::load(&model_root).expect("load model");
    let result = model
        .transcribe_wav_path(&path)
        .expect("transcribe silence");
    assert!(
        result.split_whitespace().count() <= 3,
        "Silence produced too many words: {result:?}"
    );
}

#[test]
#[ignore]
fn transcribe_sine_wave_returns_empty_or_short() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sine.wav");
    let samples = generate_sine(440.0, 2.0, 16000);
    write_wav(&path, &samples, 16000);

    let model_root = std::path::PathBuf::from(
        std::env::var("WARBLE_MODEL_ROOT").expect("set WARBLE_MODEL_ROOT"),
    );
    let mut model = warble_lib::parakeet::ParakeetTdt::load(&model_root).expect("load model");
    let result = model.transcribe_wav_path(&path).expect("transcribe sine");
    assert!(
        result.split_whitespace().count() <= 5,
        "Pure tone produced too many words: {result:?}"
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore]
fn transcribe_macos_ci_fixture_cpu_matches_expected_words() {
    run_macos_ci_fixture(MacosFixtureRuntime::Cpu);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore]
fn transcribe_macos_ci_fixture_webgpu_matches_expected_words() {
    run_macos_ci_fixture(MacosFixtureRuntime::Webgpu);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore]
fn transcribe_macos_ci_fixture_coreml_matches_expected_words() {
    if !coreml_fixture_enabled() {
        eprintln!(
            "fixture skipped: runtime=coreml reason=disabled set WARBLE_ENABLE_COREML_FIXTURE=1 to enable"
        );
        return;
    }

    run_macos_ci_fixture(MacosFixtureRuntime::Coreml);
}
