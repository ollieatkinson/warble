use anyhow::{anyhow, Context, Result};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
#[cfg(target_os = "windows")]
use ort::execution_providers::{CPUExecutionProvider, DirectMLExecutionProvider};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::state::{Settings, SystemProfile};

const EOU_MODEL_ID: &str = "parakeet-eou";
const SAMPLE_RATE: u32 = 16_000;
const N_FFT: usize = 512;
const WIN_LENGTH: usize = 400;
const HOP_LENGTH: usize = 160;
const N_MELS: usize = 128;
const PREEMPHASIS: f32 = 0.97;
const LOG_ZERO_GUARD: f32 = 5.960_464_5e-8;
const EOU_CHUNK_SAMPLES: usize = 2_560;
const EOU_BUFFER_SIZE_SAMPLES: usize = SAMPLE_RATE as usize * 4;
const EOU_MIN_BUFFER_SAMPLES: usize = SAMPLE_RATE as usize;
const EOU_PRE_ENCODE_CACHE_FRAMES: usize = 9;
const EOU_FRAMES_PER_CHUNK: usize = 16;
const EOU_ENCODER_CACHE_LAYERS: usize = 17;
const EOU_ENCODER_CHANNEL_CACHE_FRAMES: usize = 70;
const EOU_ENCODER_CHANNEL_DIM: usize = 512;
const EOU_ENCODER_TIME_CACHE_FRAMES: usize = 8;
const EOU_DECODER_STATE_DIM: usize = 640;
const EOU_MAX_TOKENS_PER_STEP: usize = 5;

pub(crate) struct StreamingPreviewConfig {
    pub(crate) model_path: PathBuf,
    directml_enabled: bool,
}

pub(crate) fn resolve_streaming_preview_config(
    settings: &Settings,
    system_profile: &SystemProfile,
) -> Option<StreamingPreviewConfig> {
    let model_path = settings.installed_model_paths.get(EOU_MODEL_ID)?;
    let model_path = PathBuf::from(model_path);
    if !eou_model_ready_in_dir(&model_path) {
        return None;
    }

    Some(StreamingPreviewConfig {
        model_path,
        directml_enabled: system_profile.directml_available,
    })
}

pub(crate) struct StreamingPreviewEngine {
    backend: ParakeetEouPreview,
    pending_audio: Vec<f32>,
    transcript: String,
}

impl StreamingPreviewEngine {
    pub(crate) fn load(config: &StreamingPreviewConfig) -> Result<Self> {
        Ok(Self {
            backend: ParakeetEouPreview::load(&config.model_path, config.directml_enabled)?,
            pending_audio: Vec::new(),
            transcript: String::new(),
        })
    }

    pub(crate) fn push_audio(&mut self, audio_16khz: &[f32]) -> Result<Option<String>> {
        if audio_16khz.is_empty() {
            return Ok(None);
        }

        self.pending_audio.extend_from_slice(audio_16khz);
        let mut changed = false;

        while self.pending_audio.len() >= EOU_CHUNK_SAMPLES {
            let chunk = self.pending_audio[..EOU_CHUNK_SAMPLES].to_vec();
            self.pending_audio.drain(..EOU_CHUNK_SAMPLES);
            let delta = self.backend.transcribe_chunk(&chunk)?;

            if !delta.text.is_empty() {
                self.transcript.push_str(&delta.text);
                changed = true;
            }

            if delta.ended_utterance {
                let trimmed = self.transcript.trim_end().to_string();
                if !trimmed.is_empty() {
                    self.transcript = trimmed;
                    if !self.transcript.ends_with('\n') {
                        self.transcript.push('\n');
                        changed = true;
                    }
                }
            }
        }

        if changed {
            Ok(Some(self.transcript.clone()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default)]
struct StreamingDelta {
    text: String,
    ended_utterance: bool,
}

struct ParakeetEouPreview {
    encoder: Session,
    decoder_joint: Session,
    tokenizer: Tokenizer,
    feature_extractor: StreamingFeatureExtractor,
    encoder_cache: EncoderCache,
    decoder_state_1: Vec<f32>,
    decoder_state_2: Vec<f32>,
    last_token: i32,
    blank_id: i32,
    eou_id: i32,
    audio_buffer: VecDeque<f32>,
}

impl ParakeetEouPreview {
    fn load(model_dir: &Path, directml_enabled: bool) -> Result<Self> {
        if !eou_model_ready_in_dir(model_dir) {
            return Err(anyhow!(
                "Streaming preview model is missing at {}",
                model_dir.display()
            ));
        }

        let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))
            .map_err(|error| anyhow!("failed to load streaming tokenizer: {error}"))?;
        let vocab_size = tokenizer.get_vocab_size(true);
        let blank_id = if vocab_size > 1_000 {
            vocab_size.saturating_sub(1) as i32
        } else {
            1_026
        };
        let eou_id = tokenizer
            .token_to_id("<EOU>")
            .map(|id| id as i32)
            .unwrap_or(1_024);

        Ok(Self {
            encoder: load_streaming_session(&model_dir.join("encoder.onnx"), directml_enabled)
                .context("failed to load streaming encoder")?,
            decoder_joint: load_streaming_session(
                &model_dir.join("decoder_joint.onnx"),
                directml_enabled,
            )
            .context("failed to load streaming decoder")?,
            tokenizer,
            feature_extractor: StreamingFeatureExtractor::new(),
            encoder_cache: EncoderCache::new(),
            decoder_state_1: vec![0.0; EOU_DECODER_STATE_DIM],
            decoder_state_2: vec![0.0; EOU_DECODER_STATE_DIM],
            last_token: blank_id,
            blank_id,
            eou_id,
            audio_buffer: VecDeque::with_capacity(EOU_BUFFER_SIZE_SAMPLES),
        })
    }

    fn transcribe_chunk(&mut self, chunk: &[f32]) -> Result<StreamingDelta> {
        self.audio_buffer.extend(chunk.iter().copied());
        while self.audio_buffer.len() > EOU_BUFFER_SIZE_SAMPLES {
            self.audio_buffer.pop_front();
        }

        if self.audio_buffer.len() < EOU_MIN_BUFFER_SAMPLES {
            return Ok(StreamingDelta::default());
        }

        let buffer_slice = self.audio_buffer.iter().copied().collect::<Vec<_>>();
        let full_features = self.feature_extractor.extract(&buffer_slice);
        let total_frames = full_features.frame_count;
        if total_frames == 0 {
            return Ok(StreamingDelta::default());
        }

        let slice_len = EOU_PRE_ENCODE_CACHE_FRAMES + EOU_FRAMES_PER_CHUNK;
        let start_frame = total_frames.saturating_sub(slice_len);
        let features = full_features.slice_recent(start_frame, slice_len);
        let encoded = self.run_encoder(&features)?;

        let mut delta = StreamingDelta::default();
        for frame_index in 0..encoded.time_steps {
            let frame = encoded.step(frame_index);
            let frame_delta = self.run_decoder_frame(&frame)?;
            if !frame_delta.text.is_empty() {
                delta.text.push_str(&frame_delta.text);
            }
            if frame_delta.ended_utterance {
                delta.ended_utterance = true;
            }
        }

        Ok(delta)
    }

    fn run_encoder(&mut self, features: &ExtractedFeatures) -> Result<EncodedAudio> {
        let feature_tensor = TensorRef::from_array_view((
            [1usize, N_MELS, features.frame_count],
            features.data.as_slice(),
        ))
        .context("failed to build streaming feature tensor")?;
        let feature_length = [features.frame_count as i64];
        let feature_length_tensor = TensorRef::from_array_view(([1usize], &feature_length[..]))
            .context("failed to build streaming feature length tensor")?;
        let channel_cache_tensor = TensorRef::from_array_view(
            (
                [
                    EOU_ENCODER_CACHE_LAYERS,
                    1usize,
                    EOU_ENCODER_CHANNEL_CACHE_FRAMES,
                    EOU_ENCODER_CHANNEL_DIM,
                ],
                self.encoder_cache.channel.as_slice(),
            ),
        )
        .context("failed to build streaming channel cache tensor")?;
        let time_cache_tensor = TensorRef::from_array_view(
            (
                [
                    EOU_ENCODER_CACHE_LAYERS,
                    1usize,
                    EOU_ENCODER_CHANNEL_DIM,
                    EOU_ENCODER_TIME_CACHE_FRAMES,
                ],
                self.encoder_cache.time.as_slice(),
            ),
        )
        .context("failed to build streaming time cache tensor")?;
        let channel_len_tensor =
            TensorRef::from_array_view(([1usize], &self.encoder_cache.channel_len[..]))
                .context("failed to build streaming cache length tensor")?;

        let outputs = self
            .encoder
            .run(inputs![
                "audio_signal" => feature_tensor,
                "length" => feature_length_tensor,
                "cache_last_channel" => channel_cache_tensor,
                "cache_last_time" => time_cache_tensor,
                "cache_last_channel_len" => channel_len_tensor
            ])
            .context("streaming encoder inference failed")?;

        let (shape, data) = outputs["outputs"]
            .try_extract_tensor::<f32>()
            .context("failed to extract streaming encoder outputs")?;
        let dims = shape_to_vec(shape);
        if dims.len() != 3 || dims[0] != 1 {
            return Err(anyhow!("unexpected streaming encoder output shape: {dims:?}"));
        }

        let (_, new_channel_cache) = outputs["new_cache_last_channel"]
            .try_extract_tensor::<f32>()
            .context("failed to extract streaming channel cache")?;
        let (_, new_time_cache) = outputs["new_cache_last_time"]
            .try_extract_tensor::<f32>()
            .context("failed to extract streaming time cache")?;
        let (_, new_channel_len) = outputs["new_cache_last_channel_len"]
            .try_extract_tensor::<i64>()
            .context("failed to extract streaming cache length")?;

        self.encoder_cache.channel.clear();
        self.encoder_cache
            .channel
            .extend_from_slice(new_channel_cache);
        self.encoder_cache.time.clear();
        self.encoder_cache.time.extend_from_slice(new_time_cache);
        self.encoder_cache.channel_len[0] = new_channel_len.first().copied().unwrap_or(0);

        Ok(EncodedAudio {
            data: data.to_vec(),
            hidden_size: dims[1],
            time_steps: dims[2],
        })
    }

    fn run_decoder_frame(&mut self, encoder_frame: &[f32]) -> Result<StreamingDelta> {
        let mut delta = StreamingDelta::default();

        for _ in 0..EOU_MAX_TOKENS_PER_STEP {
            let (token_id, next_state_1, next_state_2) = {
                let encoder_tensor = TensorRef::from_array_view((
                    [1usize, EOU_ENCODER_CHANNEL_DIM, 1usize],
                    encoder_frame,
                ))
                .context("failed to build streaming decoder frame tensor")?;
                let targets = [self.last_token];
                let targets_tensor = TensorRef::from_array_view(([1usize, 1usize], &targets[..]))
                    .context("failed to build streaming decoder target tensor")?;
                let target_length = [1i32];
                let target_length_tensor =
                    TensorRef::from_array_view(([1usize], &target_length[..]))
                        .context("failed to build streaming decoder target length tensor")?;
                let state_1_tensor = TensorRef::from_array_view(
                    ([1usize, 1usize, EOU_DECODER_STATE_DIM], self.decoder_state_1.as_slice()),
                )
                .context("failed to build streaming decoder state tensor 1")?;
                let state_2_tensor = TensorRef::from_array_view(
                    ([1usize, 1usize, EOU_DECODER_STATE_DIM], self.decoder_state_2.as_slice()),
                )
                .context("failed to build streaming decoder state tensor 2")?;

                let outputs = self
                    .decoder_joint
                    .run(inputs![
                        "encoder_outputs" => encoder_tensor,
                        "targets" => targets_tensor,
                        "target_length" => target_length_tensor,
                        "input_states_1" => state_1_tensor,
                        "input_states_2" => state_2_tensor
                    ])
                    .context("streaming decoder inference failed")?;

                let (logits_shape, logits) = outputs["outputs"]
                    .try_extract_tensor::<f32>()
                    .context("failed to extract streaming logits")?;
                let (_, next_state_1) = outputs["output_states_1"]
                    .try_extract_tensor::<f32>()
                    .context("failed to extract streaming decoder state 1")?;
                let (_, next_state_2) = outputs["output_states_2"]
                    .try_extract_tensor::<f32>()
                    .context("failed to extract streaming decoder state 2")?;

                let dims = shape_to_vec(logits_shape);
                let vocab_size = dims.last().copied().unwrap_or_default();
                if vocab_size == 0 || logits.len() < vocab_size {
                    return Err(anyhow!("unexpected streaming logits shape: {dims:?}"));
                }

                (
                    argmax(&logits[..vocab_size]) as i32,
                    next_state_1.to_vec(),
                    next_state_2.to_vec(),
                )
            };

            if token_id == 0 || token_id == self.blank_id {
                break;
            }

            if token_id == self.eou_id {
                self.reset_decoder_state();
                delta.ended_utterance = true;
                break;
            }

            self.decoder_state_1.clear();
            self.decoder_state_1.extend_from_slice(&next_state_1);
            self.decoder_state_2.clear();
            self.decoder_state_2.extend_from_slice(&next_state_2);
            self.last_token = token_id;

            if token_id >= 0 && (token_id as usize) < self.tokenizer.get_vocab_size(true) {
                if let Ok(text) = self.tokenizer.decode(&[token_id as u32], true) {
                    delta.text.push_str(&text);
                }
            } else {
                break;
            }
        }

        Ok(delta)
    }

    fn reset_decoder_state(&mut self) {
        self.decoder_state_1.fill(0.0);
        self.decoder_state_2.fill(0.0);
        self.last_token = self.blank_id;
    }
}

#[derive(Default)]
struct EncoderCache {
    channel: Vec<f32>,
    time: Vec<f32>,
    channel_len: [i64; 1],
}

impl EncoderCache {
    fn new() -> Self {
        Self {
            channel: vec![
                0.0;
                EOU_ENCODER_CACHE_LAYERS
                    * EOU_ENCODER_CHANNEL_CACHE_FRAMES
                    * EOU_ENCODER_CHANNEL_DIM
            ],
            time: vec![
                0.0;
                EOU_ENCODER_CACHE_LAYERS
                    * EOU_ENCODER_CHANNEL_DIM
                    * EOU_ENCODER_TIME_CACHE_FRAMES
            ],
            channel_len: [0],
        }
    }
}

struct EncodedAudio {
    data: Vec<f32>,
    hidden_size: usize,
    time_steps: usize,
}

impl EncodedAudio {
    fn step(&self, step: usize) -> Vec<f32> {
        let clamped_step = step.min(self.time_steps.saturating_sub(1));
        let mut frame = vec![0.0f32; self.hidden_size];
        for hidden in 0..self.hidden_size {
            frame[hidden] = self.data[hidden * self.time_steps + clamped_step];
        }
        frame
    }
}

struct ExtractedFeatures {
    data: Vec<f32>,
    frame_count: usize,
}

impl ExtractedFeatures {
    fn slice_recent(&self, start_frame: usize, max_frames: usize) -> Self {
        let end_frame = (start_frame + max_frames).min(self.frame_count);
        let sliced_frame_count = end_frame.saturating_sub(start_frame);
        let mut sliced = vec![0.0f32; N_MELS * sliced_frame_count];

        for mel in 0..N_MELS {
            let source_start = mel * self.frame_count + start_frame;
            let source_end = mel * self.frame_count + end_frame;
            let target_start = mel * sliced_frame_count;
            sliced[target_start..target_start + sliced_frame_count]
                .copy_from_slice(&self.data[source_start..source_end]);
        }

        Self {
            data: sliced,
            frame_count: sliced_frame_count,
        }
    }
}

struct StreamingFeatureExtractor {
    window: Vec<f32>,
    mel_banks: Vec<Vec<f32>>,
    bit_reverse: Vec<usize>,
    twiddle_cos: Vec<f32>,
    twiddle_sin: Vec<f32>,
}

impl StreamingFeatureExtractor {
    fn new() -> Self {
        let (twiddle_cos, twiddle_sin) = create_twiddle_tables(N_FFT);
        Self {
            window: build_centered_hann_window(),
            mel_banks: build_mel_filter_bank(N_MELS),
            bit_reverse: create_bit_reverse_table(N_FFT),
            twiddle_cos,
            twiddle_sin,
        }
    }

    fn extract(&self, audio: &[f32]) -> ExtractedFeatures {
        let mut preemphasized = vec![0.0f32; audio.len()];
        if let Some(first) = audio.first() {
            preemphasized[0] = *first;
            for index in 1..audio.len() {
                preemphasized[index] = audio[index] - PREEMPHASIS * audio[index - 1];
            }
        }

        let mut padded = vec![0.0f32; preemphasized.len() + N_FFT];
        padded[N_FFT / 2..N_FFT / 2 + preemphasized.len()].copy_from_slice(&preemphasized);

        let frame_count = usize::max(1, (padded.len().saturating_sub(N_FFT)) / HOP_LENGTH + 1);
        let mut features = vec![0.0f32; frame_count * N_MELS];
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
                features[mel_index * frame_count + frame] = (energy + LOG_ZERO_GUARD).ln();
            }
        }

        ExtractedFeatures {
            data: features,
            frame_count,
        }
    }
}

fn eou_model_ready_in_dir(model_dir: &Path) -> bool {
    model_dir.join("encoder.onnx").exists()
        && model_dir.join("decoder_joint.onnx").exists()
        && model_dir.join("tokenizer.json").exists()
}

fn load_streaming_session(path: &Path, directml_enabled: bool) -> Result<Session> {
    let mut builder = Session::builder()
        .context("failed to create ONNX session builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|error| anyhow!("failed to configure ONNX session: {error}"))?;

    #[cfg(target_os = "windows")]
    if directml_enabled {
        builder = builder.with_execution_providers([
            DirectMLExecutionProvider::default().build(),
            CPUExecutionProvider::default().build().error_on_failure(),
        ])?;
    }

    builder
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

fn hz_to_mel(hz: f32) -> f32 {
    2_595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10f32.powf(mel / 2_595.0) - 1.0)
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
