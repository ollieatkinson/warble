use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig, SupportedStreamConfig,
};
use std::sync::{Arc, Mutex};

use crate::state::SourceInfo;

fn make_source_id(index: usize, name: &str) -> String {
    format!("{index}:{name}")
}

pub(crate) fn enumerate_sources() -> Vec<SourceInfo> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    host.input_devices()
        .map(|devices| {
            devices
                .enumerate()
                .map(|(index, device)| {
                    let name = device
                        .name()
                        .unwrap_or_else(|_| format!("Input {}", index + 1));
                    let config = device.default_input_config().ok();
                    SourceInfo {
                        id: make_source_id(index, &name),
                        name: name.clone(),
                        sample_rate: config.as_ref().map(|cfg| cfg.sample_rate().0).unwrap_or(0),
                        channels: config.as_ref().map(|cfg| cfg.channels()).unwrap_or(0),
                        is_default: default_name
                            .as_ref()
                            .map(|value| value == &name)
                            .unwrap_or(false),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn resolve_selected_device(
    selected_source_id: Option<&str>,
) -> Result<(cpal::Device, SourceInfo, SupportedStreamConfig)> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .context("failed to read input devices")?;

    let mut first_candidate: Option<(cpal::Device, SourceInfo, SupportedStreamConfig)> = None;

    for (index, device) in devices.enumerate() {
        let name = device
            .name()
            .unwrap_or_else(|_| format!("Input {}", index + 1));
        let Ok(config) = device.default_input_config() else {
            continue;
        };

        let info = SourceInfo {
            id: make_source_id(index, &name),
            name: name.clone(),
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
            is_default: default_name
                .as_ref()
                .map(|value| value == &name)
                .unwrap_or(false),
        };

        if first_candidate.is_none() {
            first_candidate = Some((device.clone(), info.clone(), config.clone()));
        }

        if selected_source_id
            .map(|value| value == info.id)
            .unwrap_or(info.is_default)
        {
            return Ok((device, info, config));
        }
    }

    first_candidate.ok_or_else(|| anyhow!("No input devices were found"))
}

fn write_input_data<T>(input: &[T], channels: usize, destination: &Arc<Mutex<Vec<f32>>>)
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let mut samples = destination.lock().expect("recording buffer poisoned");
    for frame in input.chunks(channels) {
        let sum: f32 = frame.iter().map(|sample| f32::from_sample(*sample)).sum();
        samples.push(sum / channels as f32);
    }
}

pub(crate) fn build_input_stream(
    device: &cpal::Device,
    supported_config: &SupportedStreamConfig,
) -> Result<(Stream, Arc<Mutex<Vec<f32>>>)> {
    let config = supported_config.config();
    let channels = config.channels as usize;
    let destination = Arc::new(Mutex::new(Vec::<f32>::new()));
    let error_callback = |error| eprintln!("audio stream error: {error}");

    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => build_typed_stream::<f32>(
            &device,
            &config,
            channels,
            destination.clone(),
            error_callback,
        )?,
        SampleFormat::I16 => build_typed_stream::<i16>(
            &device,
            &config,
            channels,
            destination.clone(),
            error_callback,
        )?,
        SampleFormat::U16 => build_typed_stream::<u16>(
            &device,
            &config,
            channels,
            destination.clone(),
            error_callback,
        )?,
        other => return Err(anyhow!("Unsupported sample format: {other:?}")),
    };

    Ok((stream, destination))
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    destination: Arc<Mutex<Vec<f32>>>,
    error_callback: fn(cpal::StreamError),
) -> Result<Stream>
where
    T: Sample + SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |input: &[T], _| write_input_data(input, channels, &destination),
        error_callback,
        None,
    )?;
    Ok(stream)
}
