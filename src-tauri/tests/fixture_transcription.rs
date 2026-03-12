// Fixture-based transcription tests.
//
// The helper utilities (WAV generation, scoring) are tested directly.
// Tests that require a loaded model are gated with `#[ignore]`.

use std::path::Path;

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
    assert!(
        (rms - 0.707).abs() < 0.01,
        "RMS was {rms}, expected ~0.707"
    );
}

// ---------------------------------------------------------------------------
// Word match scoring
// ---------------------------------------------------------------------------

#[test]
fn word_match_score_exact() {
    assert!(
        (word_match_score(&["hello", "world"], "Hello World") - 1.0).abs() < f64::EPSILON
    );
}

#[test]
fn word_match_score_partial() {
    assert!(
        (word_match_score(&["hello", "world"], "hello there") - 0.5).abs() < f64::EPSILON
    );
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
        std::env::var("TRANSCRIBED_MODEL_ROOT").expect("set TRANSCRIBED_MODEL_ROOT"),
    );
    let mut model =
        transcribed_lib::parakeet::ParakeetTdt::load(&model_root).expect("load model");
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
        std::env::var("TRANSCRIBED_MODEL_ROOT").expect("set TRANSCRIBED_MODEL_ROOT"),
    );
    let mut model =
        transcribed_lib::parakeet::ParakeetTdt::load(&model_root).expect("load model");
    let result = model
        .transcribe_wav_path(&path)
        .expect("transcribe sine");
    assert!(
        result.split_whitespace().count() <= 5,
        "Pure tone produced too many words: {result:?}"
    );
}
