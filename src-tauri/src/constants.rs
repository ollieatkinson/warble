pub(crate) const EVENT_SNAPSHOT: &str = "transcribed://snapshot";
pub(crate) const PERSISTED_STATE_FILE: &str = "state.json";
pub(crate) const RECORDINGS_DIR: &str = "recordings";
pub(crate) const HOLD_MIN_DURATION_MS: u64 = 250;
pub(crate) const HISTORY_LIMIT: usize = 50;
pub(crate) const DEFAULT_HOLD_SHORTCUT: &str = "F8";
pub(crate) const DEFAULT_TOGGLE_SHORTCUT: &str = "F9";
pub(crate) const DEFAULT_CLEANUP_TERMS: &[&str] = &["um", "uh", "erm", "uhm", "hmm"];
pub(crate) const LEGACY_HOLD_SHORTCUT: &str = "Ctrl+Alt+Space";
pub(crate) const LEGACY_TOGGLE_SHORTCUT: &str = "Ctrl+Alt+Shift+Space";
pub(crate) const CANCEL_SHORTCUT: &str = "Escape";
pub(crate) const INDICATOR_MARGIN: i32 = 24;
pub(crate) const LIVE_PREVIEW_INTERVAL_MS: u64 = 1_500;
pub(crate) const LIVE_STREAM_PREVIEW_INTERVAL_MS: u64 = 220;
pub(crate) const LIVE_STREAM_PREVIEW_FALLBACK_MS: u64 = 4_000;
pub(crate) const LIVE_PREVIEW_MIN_MS: u64 = 900;
pub(crate) const LIVE_PREVIEW_WINDOW_SECONDS: usize = 12;
pub(crate) const LIVE_PREVIEW_MAX_WORDS: usize = 32;
pub(crate) const LIVE_PREVIEW_RESET_AFTER_DIVERGENCE: usize = 2;
pub(crate) const LIVE_PREVIEW_DRAFT_FALLBACK_PASSES: usize = 2;
pub(crate) const LIVE_METER_INTERVAL_MS: u64 = 75;
pub(crate) const LIVE_METER_WINDOW_MS: u64 = 700;
pub(crate) const LIVE_METER_ANALYSIS_SAMPLES: usize = 2_048;
pub(crate) const LIVE_METER_BAR_COUNT: usize = 12;
pub(crate) const LIVE_METER_SILENCE_RMS_THRESHOLD: f32 = 0.0045;
pub(crate) const LIVE_METER_SILENCE_PEAK_THRESHOLD: f32 = 0.015;
pub(crate) const LIVE_METER_FULL_RMS: f32 = 0.05;
pub(crate) const LIVE_METER_FULL_PEAK: f32 = 0.18;
pub(crate) const BATCH_MODEL_AUDIO_LIMIT_MS: u64 = 5 * 60 * 1_000;
pub(crate) const INDICATOR_WINDOW_PADDING: i32 = 14;
pub(crate) const BACKGROUND_ARG: &str = "--background";
pub(crate) const TRAY_ID: &str = "main-tray";
pub(crate) const TRAY_SHOW_ID: &str = "tray-show";
pub(crate) const TRAY_HIDE_ID: &str = "tray-hide";
pub(crate) const TRAY_QUIT_ID: &str = "tray-quit";
pub(crate) const MANAGED_MODELS_DIR: &str = "catalog-models";
pub(crate) const PARAKEET_ENCODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8/resolve/main/encoder-model.int8.onnx";
pub(crate) const PARAKEET_DECODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8/resolve/main/decoder_joint-model.int8.onnx";
pub(crate) const PARAKEET_VOCAB_DOWNLOAD_URL: &str =
    "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8/resolve/main/vocab.txt";
pub(crate) const PARAKEET_CTC_MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/onnx/model_int8.onnx";
pub(crate) const PARAKEET_CTC_MODEL_DATA_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/onnx/model_int8.onnx_data";
pub(crate) const PARAKEET_CTC_CONFIG_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/config.json";
pub(crate) const PARAKEET_CTC_PREPROCESSOR_CONFIG_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/preprocessor_config.json";
pub(crate) const PARAKEET_CTC_SPECIAL_TOKENS_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/special_tokens_map.json";
pub(crate) const PARAKEET_CTC_TOKENIZER_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/tokenizer.json";
pub(crate) const PARAKEET_CTC_TOKENIZER_CONFIG_DOWNLOAD_URL: &str =
    "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/resolve/main/tokenizer_config.json";
pub(crate) const PARAKEET_EOU_ENCODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/realtime_eou_120m-v1-onnx/encoder.onnx";
pub(crate) const PARAKEET_EOU_DECODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/realtime_eou_120m-v1-onnx/decoder_joint.onnx";
pub(crate) const PARAKEET_EOU_TOKENIZER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/realtime_eou_120m-v1-onnx/tokenizer.json";
pub(crate) const NEMOTRON_ENCODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/nemotron-speech-streaming-en-0.6b/encoder.onnx";
pub(crate) const NEMOTRON_ENCODER_DATA_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/nemotron-speech-streaming-en-0.6b/encoder.onnx.data";
pub(crate) const NEMOTRON_DECODER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/nemotron-speech-streaming-en-0.6b/decoder_joint.onnx";
pub(crate) const NEMOTRON_TOKENIZER_DOWNLOAD_URL: &str =
    "https://huggingface.co/altunenes/parakeet-rs/resolve/main/nemotron-speech-streaming-en-0.6b/tokenizer.model";
