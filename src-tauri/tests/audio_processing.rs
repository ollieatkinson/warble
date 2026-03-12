// Integration tests for audio processing functions exposed via the public API.
//
// The `parakeet` module is the only public module in warble_lib, so we
// exercise its resampling, model-readiness, and detection helpers here.

use warble_lib::parakeet;

// ---------------------------------------------------------------------------
// resample_to_16khz
// ---------------------------------------------------------------------------

#[test]
fn resample_identity_at_16khz() {
    let input: Vec<f32> = (0..160).map(|i| i as f32 / 160.0).collect();
    let output = parakeet::resample_to_16khz(&input, 16_000);
    assert_eq!(output.len(), input.len());
    for (a, b) in input.iter().zip(output.iter()) {
        assert!((*a - *b).abs() < 1e-6f32, "mismatch: {a} vs {b}");
    }
}

#[test]
fn resample_empty_input_returns_empty() {
    let output = parakeet::resample_to_16khz(&[], 48_000);
    assert!(output.is_empty());
}

#[test]
fn resample_downsamples_48khz() {
    let input = vec![0.5f32; 4800];
    let output = parakeet::resample_to_16khz(&input, 48_000);
    let expected_len = (4800.0_f64 * 16_000.0 / 48_000.0).round() as usize;
    assert_eq!(output.len(), expected_len);
}

#[test]
fn resample_preserves_dc_offset() {
    let dc = 0.42f32;
    let input = vec![dc; 9600];
    let output = parakeet::resample_to_16khz(&input, 48_000);
    for sample in &output {
        assert!(
            (*sample - dc).abs() < 1e-5f32,
            "DC offset not preserved: got {sample}"
        );
    }
}

#[test]
fn resample_upsamples_8khz() {
    let input = vec![1.0f32; 800];
    let output = parakeet::resample_to_16khz(&input, 8_000);
    let expected_len = (800.0_f64 * 16_000.0 / 8_000.0).round() as usize;
    assert_eq!(output.len(), expected_len);
    for sample in &output {
        assert!((*sample - 1.0f32).abs() < 1e-5f32);
    }
}

// ---------------------------------------------------------------------------
// model_ready_at / model_ready_in_dir
// ---------------------------------------------------------------------------

#[test]
fn model_ready_in_dir_empty_dir_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!parakeet::model_ready_in_dir(dir.path()));
}

#[test]
fn model_ready_in_dir_complete_returns_true() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vocab.txt"), "").unwrap();
    std::fs::write(dir.path().join("encoder-model.onnx"), "").unwrap();
    std::fs::write(dir.path().join("decoder_joint-model.onnx"), "").unwrap();
    assert!(parakeet::model_ready_in_dir(dir.path()));
}

#[test]
fn model_ready_at_checks_named_subdirectory() {
    let root = tempfile::tempdir().unwrap();
    assert!(!parakeet::model_ready_at(root.path()));

    let model_dir = root.path().join(parakeet::MODEL_ID);
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(model_dir.join("vocab.txt"), "").unwrap();
    std::fs::write(model_dir.join("encoder.onnx"), "").unwrap();
    std::fs::write(model_dir.join("decoder_joint.onnx"), "").unwrap();
    assert!(parakeet::model_ready_at(root.path()));
}

// ---------------------------------------------------------------------------
// ctc_model_ready_at / ctc_model_ready_in_dir
// ---------------------------------------------------------------------------

#[test]
fn ctc_model_ready_in_dir_empty_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!parakeet::ctc_model_ready_in_dir(dir.path()));
}

#[test]
fn ctc_model_ready_in_dir_complete_returns_true() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("tokenizer.json"), "").unwrap();
    std::fs::write(dir.path().join("model.onnx"), "").unwrap();
    assert!(parakeet::ctc_model_ready_in_dir(dir.path()));
}

#[test]
fn ctc_model_ready_at_checks_named_subdirectory() {
    let root = tempfile::tempdir().unwrap();
    assert!(!parakeet::ctc_model_ready_at(root.path()));

    let ctc_dir = root.path().join(parakeet::CTC_MODEL_ID);
    std::fs::create_dir_all(&ctc_dir).unwrap();
    std::fs::write(ctc_dir.join("tokenizer.json"), "").unwrap();
    std::fs::write(ctc_dir.join("model.onnx"), "").unwrap();
    assert!(parakeet::ctc_model_ready_at(root.path()));
}

// ---------------------------------------------------------------------------
// detect_model_dir
// ---------------------------------------------------------------------------

#[test]
fn detect_model_dir_returns_none_for_empty() {
    let root = tempfile::tempdir().unwrap();
    assert!(parakeet::detect_model_dir(root.path()).is_none());
}

#[test]
fn detect_model_dir_finds_tdt() {
    let root = tempfile::tempdir().unwrap();
    let model_dir = root.path().join(parakeet::MODEL_ID);
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(model_dir.join("vocab.txt"), "").unwrap();
    std::fs::write(model_dir.join("encoder-model.onnx"), "").unwrap();
    std::fs::write(model_dir.join("decoder_joint-model.onnx"), "").unwrap();

    let (family, path) = parakeet::detect_model_dir(root.path()).unwrap();
    assert_eq!(family, parakeet::TranscriptionFamily::Tdt);
    assert_eq!(path, model_dir);
}

#[test]
fn detect_model_dir_finds_ctc() {
    let root = tempfile::tempdir().unwrap();
    let ctc_dir = root.path().join(parakeet::CTC_MODEL_ID);
    std::fs::create_dir_all(&ctc_dir).unwrap();
    std::fs::write(ctc_dir.join("tokenizer.json"), "").unwrap();
    std::fs::write(ctc_dir.join("model.onnx"), "").unwrap();

    let (family, _) = parakeet::detect_model_dir(root.path()).unwrap();
    assert_eq!(family, parakeet::TranscriptionFamily::Ctc);
}

#[test]
fn detect_model_dir_prefers_tdt_over_ctc() {
    let root = tempfile::tempdir().unwrap();

    // Set up both TDT and CTC
    let tdt_dir = root.path().join(parakeet::MODEL_ID);
    std::fs::create_dir_all(&tdt_dir).unwrap();
    std::fs::write(tdt_dir.join("vocab.txt"), "").unwrap();
    std::fs::write(tdt_dir.join("encoder-model.onnx"), "").unwrap();
    std::fs::write(tdt_dir.join("decoder_joint-model.onnx"), "").unwrap();

    let ctc_dir = root.path().join(parakeet::CTC_MODEL_ID);
    std::fs::create_dir_all(&ctc_dir).unwrap();
    std::fs::write(ctc_dir.join("tokenizer.json"), "").unwrap();
    std::fs::write(ctc_dir.join("model.onnx"), "").unwrap();

    let (family, _) = parakeet::detect_model_dir(root.path()).unwrap();
    assert_eq!(family, parakeet::TranscriptionFamily::Tdt);
}

// ---------------------------------------------------------------------------
// Constants are accessible
// ---------------------------------------------------------------------------

#[test]
fn sample_rate_is_16khz() {
    assert_eq!(parakeet::SAMPLE_RATE, 16_000);
}
