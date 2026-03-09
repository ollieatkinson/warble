use anyhow::{anyhow, bail, Context, Result};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const SAMPLE_RATE: u32 = 16_000;
pub const MODEL_ID: &str = "parakeet-tdt-0.6b-v3-int8";
pub const CTC_MODEL_ID: &str = "parakeet-ctc-0.6b";

const TDT_REQUIRED_FILES: &[&str] = &["vocab.txt"];
const TDT_ENCODER_CANDIDATES: &[&str] =
    &["encoder-model.onnx", "encoder-model.int8.onnx", "encoder.onnx"];
const TDT_DECODER_CANDIDATES: &[&str] = &[
    "decoder_joint-model.onnx",
    "decoder_joint-model.int8.onnx",
    "decoder_joint.onnx",
    "decoder-model.onnx",
];
const CTC_MODEL_CANDIDATES: &[&str] =
    &["model.onnx", "model_fp16.onnx", "model_int8.onnx", "model_q4.onnx"];

const N_FFT: usize = 512;
const WIN_LENGTH: usize = 400;
const HOP_LENGTH: usize = 160;
const PREEMPHASIS: f32 = 0.97;
const LOG_ZERO_GUARD: f32 = 5.960_464_5e-8;
const DEFAULT_FEATURE_SIZE: usize = 128;
const DEFAULT_MAX_TOKENS_PER_STEP: usize = 10;
const DECODER_STATE_LAYERS: usize = 2;
const DECODER_STATE_HIDDEN: usize = 640;

fn has_any_file(dir: &Path, candidates: &[&str]) -> bool {
    candidates.iter().any(|filename| dir.join(filename).exists())
}

pub fn model_ready_at(root: &Path) -> bool {
    model_ready_in_dir(&root.join(MODEL_ID))
}

pub fn model_ready_in_dir(model_dir: &Path) -> bool {
    TDT_REQUIRED_FILES
        .iter()
        .all(|filename| model_dir.join(filename).exists())
        && has_any_file(model_dir, TDT_ENCODER_CANDIDATES)
        && has_any_file(model_dir, TDT_DECODER_CANDIDATES)
}

pub fn ctc_model_ready_at(root: &Path) -> bool {
    ctc_model_ready_in_dir(&root.join(CTC_MODEL_ID))
}

pub fn ctc_model_ready_in_dir(model_dir: &Path) -> bool {
    model_dir.join("tokenizer.json").exists() && has_any_file(model_dir, CTC_MODEL_CANDIDATES)
}

pub fn detect_model_dir(root: &Path) -> Option<(TranscriptionFamily, PathBuf)> {
    let tdt_dir = root.join(MODEL_ID);
    if model_ready_in_dir(&tdt_dir) {
        return Some((TranscriptionFamily::Tdt, tdt_dir));
    }

    let ctc_dir = root.join(CTC_MODEL_ID);
    if ctc_model_ready_in_dir(&ctc_dir) {
        return Some((TranscriptionFamily::Ctc, ctc_dir));
    }

    if model_ready_in_dir(root) {
        return Some((TranscriptionFamily::Tdt, root.to_path_buf()));
    }

    if ctc_model_ready_in_dir(root) {
        return Some((TranscriptionFamily::Ctc, root.to_path_buf()));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptionFamily {
    Tdt,
    Ctc,
}

#[derive(Debug)]
pub struct ParakeetTdt {
    preprocessor: Option<Session>,
    encoder: Session,
    decoder: Session,
    feature_extractor: ParakeetFeatureExtractor,
    vocab: Vocabulary,
    max_tokens_per_step: usize,
}

impl ParakeetTdt {
    pub fn load(model_root: &Path) -> Result<Self> {
        let model_dir = model_root.join(MODEL_ID);
        Self::load_from_dir(&model_dir)
    }

    pub fn load_from_dir(model_dir: &Path) -> Result<Self> {
        if !model_ready_in_dir(model_dir) {
            bail!("Parakeet TDT model is missing at {}", model_dir.display());
        }

        let config = fs::read(model_dir.join("config.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ModelConfig>(&bytes).ok())
            .unwrap_or_default();
        let vocab = Vocabulary::load(&model_dir.join("vocab.txt"))?;
        let feature_size = config.features_size.unwrap_or(DEFAULT_FEATURE_SIZE);

        let preprocessor = if model_dir.join("nemo128.onnx").exists() {
            load_session(&model_dir.join("nemo128.onnx")).ok()
        } else {
            None
        };

        Ok(Self {
            preprocessor,
            encoder: load_session(&find_existing_path(model_dir, TDT_ENCODER_CANDIDATES)?)
                .context("failed to load encoder model")?,
            decoder: load_session(&find_existing_path(model_dir, TDT_DECODER_CANDIDATES)?)
                .context("failed to load decoder model")?,
            feature_extractor: ParakeetFeatureExtractor::new(feature_size),
            vocab,
            max_tokens_per_step: config
                .max_tokens_per_step
                .unwrap_or(DEFAULT_MAX_TOKENS_PER_STEP),
        })
    }

    pub fn transcribe_audio(&mut self, audio: &[f32]) -> Result<String> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let features = self.extract_features(audio)?;
        let encoded = self.encode(&features)?;
        self.decode(&encoded)
    }

    pub fn transcribe_wav_path(&mut self, wav_path: &Path) -> Result<String> {
        let audio = read_wav_mono(wav_path)?;
        self.transcribe_audio(&audio)
    }

    fn extract_features(&mut self, audio: &[f32]) -> Result<ExtractedFeatures> {
        let Some(preprocessor) = self.preprocessor.as_mut() else {
            return Ok(self.feature_extractor.extract(audio));
        };

        let lengths = [audio.len() as i64];
        let waveform_tensor = TensorRef::from_array_view(([1usize, audio.len()], audio))
            .context("failed to build waveform tensor")?;
        let length_tensor = TensorRef::from_array_view(([1usize], &lengths[..]))
            .context("failed to build waveform length tensor")?;

        let outputs = match preprocessor.run(inputs![waveform_tensor, length_tensor]) {
            Ok(outputs) => outputs,
            Err(_) => return Ok(self.feature_extractor.extract(audio)),
        };

        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .context("failed to extract preprocessor features")?;
        let (_, lengths) = outputs[1]
            .try_extract_tensor::<i64>()
            .context("failed to extract preprocessor lengths")?;

        let dims = shape_to_vec(shape);
        let frame_count = *dims.get(2).unwrap_or(&1);
        let features_length = lengths
            .first()
            .and_then(|value| usize::try_from(*value).ok())
            .unwrap_or(frame_count)
            .clamp(1, frame_count.max(1));

        Ok(ExtractedFeatures {
            input_features: data.to_vec(),
            input_shape: [1, *dims.get(1).unwrap_or(&DEFAULT_FEATURE_SIZE), frame_count.max(1)],
            features_length,
        })
    }

    fn encode(&mut self, features: &ExtractedFeatures) -> Result<EncodedAudio> {
        let length = [features.features_length as i64];
        let features_tensor =
            TensorRef::from_array_view((features.input_shape, features.input_features.as_slice()))
                .context("failed to build encoder feature tensor")?;
        let length_tensor = TensorRef::from_array_view(([1usize], &length[..]))
            .context("failed to build encoder length tensor")?;

        let outputs = self
            .encoder
            .run(inputs![features_tensor, length_tensor])
            .context("encoder inference failed")?;
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .context("failed to extract encoder outputs")?;
        let (_, encoded_lengths) = outputs[1]
            .try_extract_tensor::<i64>()
            .context("failed to extract encoded lengths")?;

        let dims = shape_to_vec(shape);
        if dims.len() != 3 || dims[0] != 1 {
            return Err(anyhow!("unexpected encoder output shape: {dims:?}"));
        }

        let hidden_size = dims[1];
        let time_steps = dims[2];
        let encoded_length = encoded_lengths
            .first()
            .and_then(|value| usize::try_from(*value).ok())
            .unwrap_or(time_steps)
            .clamp(1, time_steps.max(1));

        Ok(EncodedAudio {
            data: data.to_vec(),
            hidden_size,
            time_steps,
            encoded_length,
        })
    }

    fn decode(&mut self, encoded: &EncodedAudio) -> Result<String> {
        let mut state = DecoderState::new();
        let mut token_ids = Vec::new();
        let mut step = 0usize;
        let mut emitted_tokens = 0usize;
        let mut guard = 0usize;
        let guard_limit = encoded.encoded_length * usize::max(8, self.max_tokens_per_step * 4);

        while step < encoded.encoded_length && guard < guard_limit {
            guard += 1;

            let encoder_step = encoded.step(step);
            let encoder_tensor = TensorRef::from_array_view((
                [1usize, encoded.hidden_size, 1usize],
                encoder_step.as_slice(),
            ))
            .context("failed to build decoder encoder step tensor")?;
            let last_token_id = token_ids
                .last()
                .copied()
                .unwrap_or(self.vocab.blank_token_id as i32);
            let targets = [last_token_id];
            let target_length = [1i32];
            let targets_tensor = TensorRef::from_array_view(([1usize, 1usize], &targets[..]))
                .context("failed to build decoder targets tensor")?;
            let target_length_tensor = TensorRef::from_array_view(([1usize], &target_length[..]))
                .context("failed to build decoder target length tensor")?;
            let state1_tensor = TensorRef::from_array_view((state.shape, state.state1.as_slice()))
                .context("failed to build decoder state tensor 1")?;
            let state2_tensor = TensorRef::from_array_view((state.shape, state.state2.as_slice()))
                .context("failed to build decoder state tensor 2")?;

            let outputs = self
                .decoder
                .run(inputs![
                    encoder_tensor,
                    targets_tensor,
                    target_length_tensor,
                    state1_tensor,
                    state2_tensor
                ])
                .context("decoder inference failed")?;

            let (_, logits) = outputs[0]
                .try_extract_tensor::<f32>()
                .context("failed to extract decoder logits")?;
            let (_, state1) = outputs[2]
                .try_extract_tensor::<f32>()
                .context("failed to extract decoder state 1")?;
            let (_, state2) = outputs[3]
                .try_extract_tensor::<f32>()
                .context("failed to extract decoder state 2")?;

            let vocab_size = self.vocab.tokens.len();
            if logits.len() < vocab_size {
                return Err(anyhow!(
                    "decoder logits were smaller than the vocabulary: {} < {}",
                    logits.len(),
                    vocab_size
                ));
            }

            let token = argmax(&logits[..vocab_size]) as i32;
            let step_count = if logits.len() > vocab_size {
                argmax(&logits[vocab_size..])
            } else {
                0
            };

            if token != self.vocab.blank_token_id as i32 {
                token_ids.push(token);
                emitted_tokens += 1;
                state.state1.clear();
                state.state1.extend_from_slice(state1);
                state.state2.clear();
                state.state2.extend_from_slice(state2);
            }

            if step_count > 0 {
                step += step_count;
                emitted_tokens = 0;
            } else if token == self.vocab.blank_token_id as i32
                || emitted_tokens >= self.max_tokens_per_step
            {
                step += 1;
                emitted_tokens = 0;
            }
        }

        Ok(self.vocab.decode(&token_ids))
    }
}

pub struct ParakeetCtc {
    _private: (),
}

impl ParakeetCtc {
    pub fn load(model_root: &Path) -> Result<Self> {
        let model_dir = model_root.join(CTC_MODEL_ID);
        Self::load_from_dir(&model_dir)
    }

    pub fn load_from_dir(model_dir: &Path) -> Result<Self> {
        if !ctc_model_ready_in_dir(model_dir) {
            bail!("Parakeet CTC model is missing at {}", model_dir.display());
        }

        bail!(
            "Parakeet CTC is cataloged in the app, but the stable Rust runtime is not wired for it yet"
        )
    }

    pub fn transcribe_audio(&mut self, audio: &[f32]) -> Result<String> {
        let _ = audio;
        bail!("Parakeet CTC transcription is not available in the stable runtime yet")
    }
}

#[derive(Debug, Default, Deserialize)]
struct ModelConfig {
    features_size: Option<usize>,
    max_tokens_per_step: Option<usize>,
}

#[derive(Debug)]
struct ExtractedFeatures {
    input_features: Vec<f32>,
    input_shape: [usize; 3],
    features_length: usize,
}

#[derive(Debug)]
struct EncodedAudio {
    data: Vec<f32>,
    hidden_size: usize,
    time_steps: usize,
    encoded_length: usize,
}

impl EncodedAudio {
    fn step(&self, step: usize) -> Vec<f32> {
        let clamped_step = step.min(self.time_steps.saturating_sub(1));
        let mut vector = vec![0.0; self.hidden_size];
        for hidden in 0..self.hidden_size {
            vector[hidden] = self.data[hidden * self.time_steps + clamped_step];
        }
        vector
    }
}

#[derive(Debug)]
struct DecoderState {
    shape: [usize; 3],
    state1: Vec<f32>,
    state2: Vec<f32>,
}

impl DecoderState {
    fn new() -> Self {
        let size = DECODER_STATE_LAYERS * DECODER_STATE_HIDDEN;
        Self {
            shape: [DECODER_STATE_LAYERS, 1, DECODER_STATE_HIDDEN],
            state1: vec![0.0; size],
            state2: vec![0.0; size],
        }
    }
}

#[derive(Debug, Clone)]
struct Vocabulary {
    tokens: Vec<String>,
    blank_token_id: usize,
}

impl Vocabulary {
    fn load(vocab_path: &Path) -> Result<Self> {
        let content = fs::read_to_string(vocab_path).context("failed to read vocabulary")?;
        let mut tokens_by_id = Vec::<(usize, String)>::new();

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            let Some(separator) = line.rfind(' ') else {
                continue;
            };
            if separator == 0 {
                continue;
            }

            let id = line[separator + 1..]
                .parse::<usize>()
                .context("failed to parse vocabulary id")?;
            let token = line[..separator].replace('\u{2581}', " ");
            tokens_by_id.push((id, token));
        }

        let max_id = tokens_by_id
            .iter()
            .map(|(id, _)| *id)
            .max()
            .unwrap_or_default();
        let mut tokens = vec![String::new(); max_id + 1];
        for (id, token) in tokens_by_id {
            tokens[id] = token;
        }

        let blank_token_id = tokens
            .iter()
            .position(|token| token == "<blk>")
            .unwrap_or(tokens.len().saturating_sub(1));

        Ok(Self {
            tokens,
            blank_token_id,
        })
    }

    fn decode(&self, token_ids: &[i32]) -> String {
        let mut raw = String::new();
        for token_id in token_ids {
            let Some(token) = usize::try_from(*token_id)
                .ok()
                .and_then(|index| self.tokens.get(index))
            else {
                continue;
            };

            if token.is_empty() || token == "<unk>" || token == "<blk>" || token.starts_with("<|") {
                continue;
            }

            raw.push_str(token);
        }

        normalize_decoded_text(&raw)
    }
}

#[derive(Debug, Clone)]
struct ParakeetFeatureExtractor {
    n_mels: usize,
    window: Vec<f32>,
    mel_banks: Vec<Vec<f32>>,
    bit_reverse: Vec<usize>,
    twiddle_cos: Vec<f32>,
    twiddle_sin: Vec<f32>,
}

impl ParakeetFeatureExtractor {
    fn new(n_mels: usize) -> Self {
        let (twiddle_cos, twiddle_sin) = create_twiddle_tables(N_FFT);
        Self {
            n_mels,
            window: build_centered_hann_window(),
            mel_banks: build_mel_filter_bank(n_mels),
            bit_reverse: create_bit_reverse_table(N_FFT),
            twiddle_cos,
            twiddle_sin,
        }
    }

    fn extract(&self, audio_data: &[f32]) -> ExtractedFeatures {
        let mut preemphasized = vec![0.0f32; audio_data.len()];
        if let Some(first) = audio_data.first() {
            preemphasized[0] = *first;
            for index in 1..audio_data.len() {
                preemphasized[index] = audio_data[index] - PREEMPHASIS * audio_data[index - 1];
            }
        }

        let mut padded = vec![0.0f32; preemphasized.len() + N_FFT];
        padded[N_FFT / 2..N_FFT / 2 + preemphasized.len()].copy_from_slice(&preemphasized);

        let frame_count = usize::max(1, (padded.len().saturating_sub(N_FFT)) / HOP_LENGTH + 1);
        let features_length = usize::max(1, audio_data.len() / HOP_LENGTH);

        let mut log_mel = vec![0.0f32; frame_count * self.n_mels];
        let mut real = vec![0.0f32; N_FFT];
        let mut imag = vec![0.0f32; N_FFT];

        for frame in 0..frame_count {
            let start = frame * HOP_LENGTH;
            real.fill(0.0);
            imag.fill(0.0);

            for index in 0..N_FFT {
                real[index] = padded[start + index] * self.window[index];
            }

            fft_in_place(
                &mut real,
                &mut imag,
                &self.bit_reverse,
                &self.twiddle_cos,
                &self.twiddle_sin,
            );

            for (mel_index, bank) in self.mel_banks.iter().enumerate() {
                let mut energy = 0.0f32;
                for (bin, weight) in bank.iter().enumerate() {
                    let power = real[bin] * real[bin] + imag[bin] * imag[bin];
                    energy += power * weight;
                }
                log_mel[frame * self.n_mels + mel_index] = (energy + LOG_ZERO_GUARD).ln();
            }
        }

        let valid_frames = frame_count.min(features_length);
        let mut normalized = vec![0.0f32; self.n_mels * frame_count];
        for mel in 0..self.n_mels {
            let mut mean = 0.0f32;
            for frame in 0..valid_frames {
                mean += log_mel[frame * self.n_mels + mel];
            }
            mean /= valid_frames as f32;

            let mut variance = 0.0f32;
            for frame in 0..valid_frames {
                let delta = log_mel[frame * self.n_mels + mel] - mean;
                variance += delta * delta;
            }
            variance /= usize::max(valid_frames.saturating_sub(1), 1) as f32;
            let inv_std = 1.0 / (variance.sqrt() + 1e-5);

            for frame in 0..frame_count {
                normalized[mel * frame_count + frame] = if frame < valid_frames {
                    (log_mel[frame * self.n_mels + mel] - mean) * inv_std
                } else {
                    0.0
                };
            }
        }

        ExtractedFeatures {
            input_features: normalized,
            input_shape: [1, self.n_mels, frame_count],
            features_length: valid_frames,
        }
    }
}

fn find_existing_path(dir: &Path, candidates: &[&str]) -> Result<PathBuf> {
    for candidate in candidates {
        let path = dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!("no matching model file found in {}", dir.display()))
}

fn load_session(path: &Path) -> Result<Session> {
    Session::builder()
        .context("failed to create ONNX session builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|error| anyhow!("failed to configure ONNX session: {error}"))?
        .commit_from_file(path)
        .with_context(|| format!("failed to open {}", path.display()))
}

fn shape_to_vec(shape: &ort::tensor::Shape) -> Vec<usize> {
    shape
        .iter()
        .map(|dimension| usize::try_from(*dimension).unwrap_or_default())
        .collect()
}

fn argmax(values: &[f32]) -> usize {
    let mut best_index = 0usize;
    let mut best_value = f32::NEG_INFINITY;
    for (index, value) in values.iter().copied().enumerate() {
        if value > best_value {
            best_value = value;
            best_index = index;
        }
    }
    best_index
}

fn normalize_decoded_text(raw: &str) -> String {
    let mut normalized = String::new();
    let mut pending_space = false;

    for character in raw.trim().chars() {
        if character.is_whitespace() {
            pending_space = !normalized.is_empty();
            continue;
        }

        let suppress_space_before = matches!(
            character,
            ',' | '.' | '!' | '?' | ';' | ':' | ')' | ']' | '}' | '%' | '\''
        );

        if pending_space
            && !suppress_space_before
            && !normalized.ends_with(['(', '[', '{', '/'])
        {
            normalized.push(' ');
        }

        normalized.push(character);
        pending_space = false;
    }

    normalized
}

fn read_wav_mono(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));

    let mono = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => {
            let samples: Vec<i16> = reader
                .samples::<i16>()
                .collect::<Result<Vec<_>, _>>()
                .context("failed to read PCM16 audio")?;
            samples
                .chunks(channels)
                .map(|frame| {
                    let sum = frame
                        .iter()
                        .map(|sample| *sample as f32 / i16::MAX as f32)
                        .sum::<f32>();
                    sum / channels as f32
                })
                .collect()
        }
        (hound::SampleFormat::Float, 32) => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .context("failed to read float WAV audio")?;
            samples
                .chunks(channels)
                .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
                .collect()
        }
        _ => bail!("unsupported WAV format"),
    };

    if spec.sample_rate == SAMPLE_RATE {
        return Ok(mono);
    }

    Ok(resample_to_16khz(&mono, spec.sample_rate))
}

pub fn resample_to_16khz(input: &[f32], from_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == SAMPLE_RATE {
        return input.to_vec();
    }

    let output_len =
        ((input.len() as f64) * SAMPLE_RATE as f64 / from_rate as f64).round() as usize;
    let output_len = output_len.max(1);
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let source_position = (index as f64) * (from_rate as f64) / SAMPLE_RATE as f64;
        let left = source_position.floor() as usize;
        let right = (left + 1).min(input.len().saturating_sub(1));
        let weight = source_position - left as f64;
        let left_sample = input.get(left).copied().unwrap_or(0.0);
        let right_sample = input.get(right).copied().unwrap_or(left_sample);
        output.push(
            (left_sample as f64 * (1.0 - weight) + right_sample as f64 * weight) as f32,
        );
    }

    output
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10f32.powf(mel / 2595.0) - 1.0)
}

fn build_centered_hann_window() -> Vec<f32> {
    let mut window = vec![0.0f32; N_FFT];
    let pad = (N_FFT - WIN_LENGTH) / 2;
    for index in 0..WIN_LENGTH {
        window[pad + index] = 0.5
            - 0.5
                * ((2.0 * std::f32::consts::PI * index as f32) / (WIN_LENGTH - 1) as f32).cos();
    }
    window
}

fn build_mel_filter_bank(num_mels: usize) -> Vec<Vec<f32>> {
    let num_bins = N_FFT / 2 + 1;
    let mut banks = vec![vec![0.0f32; num_bins]; num_mels];

    let min_mel = hz_to_mel(0.0);
    let max_mel = hz_to_mel(SAMPLE_RATE as f32 / 2.0);
    let mut mel_points = vec![0.0f32; num_mels + 2];
    for (index, value) in mel_points.iter_mut().enumerate() {
        *value = min_mel + (max_mel - min_mel) * index as f32 / (num_mels + 1) as f32;
    }

    let mut bins = vec![0usize; num_mels + 2];
    for (index, value) in mel_points.iter().enumerate() {
        bins[index] =
            (((N_FFT + 1) as f32 * mel_to_hz(*value)) / SAMPLE_RATE as f32).floor() as usize;
    }

    for mel_index in 1..=num_mels {
        let left = bins[mel_index - 1];
        let center = bins[mel_index];
        let right = bins[mel_index + 1];

        if center > left {
            for bin in left..center.min(num_bins) {
                banks[mel_index - 1][bin] = (bin - left) as f32 / (center - left) as f32;
            }
        }

        if right > center {
            for bin in center..right.min(num_bins) {
                banks[mel_index - 1][bin] = (right - bin) as f32 / (right - center) as f32;
            }
        }
    }

    banks
}

fn create_bit_reverse_table(size: usize) -> Vec<usize> {
    let bits = size.ilog2();
    let mut table = vec![0usize; size];

    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index;
        let mut reversed = 0usize;
        for _ in 0..bits {
            reversed = (reversed << 1) | (value & 1);
            value >>= 1;
        }
        *slot = reversed;
    }

    table
}

fn create_twiddle_tables(size: usize) -> (Vec<f32>, Vec<f32>) {
    let half = size / 2;
    let mut cos = vec![0.0f32; half];
    let mut sin = vec![0.0f32; half];

    for index in 0..half {
        let angle = (2.0 * std::f32::consts::PI * index as f32) / size as f32;
        cos[index] = angle.cos();
        sin[index] = angle.sin();
    }

    (cos, sin)
}

fn fft_in_place(
    real: &mut [f32],
    imag: &mut [f32],
    bit_reverse: &[usize],
    cos: &[f32],
    sin: &[f32],
) {
    let size = real.len();

    for index in 0..size {
        let reversed = bit_reverse[index];
        if reversed > index {
            real.swap(index, reversed);
            imag.swap(index, reversed);
        }
    }

    let mut span = 2usize;
    while span <= size {
        let half = span / 2;
        let step = size / span;

        let mut start = 0usize;
        while start < size {
            for offset in 0..half {
                let even = start + offset;
                let odd = even + half;
                let twiddle = offset * step;
                let wr = cos[twiddle];
                let wi = sin[twiddle];
                let odd_real = real[odd];
                let odd_imag = imag[odd];
                let transformed_real = odd_real * wr + odd_imag * wi;
                let transformed_imag = odd_imag * wr - odd_real * wi;

                real[odd] = real[even] - transformed_real;
                imag[odd] = imag[even] - transformed_imag;
                real[even] += transformed_real;
                imag[even] += transformed_imag;
            }

            start += span;
        }

        span <<= 1;
    }
}
