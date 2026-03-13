use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size};

use crate::constants::{
    INDICATOR_MARGIN, INDICATOR_WINDOW_PADDING, LIVE_METER_ANALYSIS_SAMPLES, LIVE_METER_BAR_COUNT,
    LIVE_METER_FULL_PEAK, LIVE_METER_FULL_RMS, LIVE_METER_SILENCE_PEAK_THRESHOLD,
    LIVE_METER_SILENCE_RMS_THRESHOLD,
};
use crate::state::{
    AppCore, LiveTranscriptLines, LiveTranscriptWidth, OverlayAnimationStyle, OverlayPosition,
    OverlaySnapshot, Settings, SharedState,
};

// Frequency analysis
const MIN_ANALYSIS_FREQ_HZ: f32 = 120.0;
const MAX_ANALYSIS_FREQ_HZ: f32 = 5_800.0;
const NYQUIST_FRACTION: f32 = 0.82;
const ACTIVITY_EXPONENT: f32 = 0.85;
const ACTIVITY_GATE_THRESHOLD: f32 = 0.01;
const LEVEL_GATE_THRESHOLD: f32 = 0.025;

// Signal animation widths
const RADIAL_SIGNAL_WIDTH: i32 = 30;
const SPECTRUM_SIGNAL_WIDTH: i32 = 60;
const WAVEFORM_SIGNAL_WIDTH: i32 = 62;

// Transcript panel widths
const COMPACT_TRANSCRIPT_WIDTH: i32 = 256;
const BALANCED_TRANSCRIPT_WIDTH: i32 = 320;
const WIDE_TRANSCRIPT_WIDTH: i32 = 392;

// Indicator layout
const STATUS_WIDTH: i32 = 18;
const GAP_WIDTH: i32 = 10;
const TIMER_CHARACTER_COUNT: i32 = 11;
const TIMER_CHARACTER_WIDTH: i32 = 9;
const TIMER_INSET_WIDTH: i32 = 12;
const TRANSCRIPT_LINE_HEIGHT: i32 = 16;
const RADIAL_ROW_HEIGHT: i32 = 30;
const NON_RADIAL_ROW_HEIGHT: i32 = 20;
const TRANSCRIPT_GAP_HEIGHT: i32 = 9;
const SHELL_PADDING_HEIGHT: i32 = 28;
const COMPACT_INDICATOR_HEIGHT: i32 = 58;
const PILL_PADDING_WITH_TRANSCRIPT: i32 = 34;
const PILL_PADDING_WITHOUT_TRANSCRIPT: i32 = 30;
const DYNAMIC_ISLAND_MARGIN_TOP: i32 = 0;
const DEFAULT_DYNAMIC_ISLAND_GAP_WIDTH: i32 = 118;
const DEFAULT_DYNAMIC_ISLAND_GAP_HEIGHT: i32 = 32;
const DYNAMIC_ISLAND_POD_HORIZONTAL_PADDING: i32 = 28;
const DYNAMIC_ISLAND_TIMER_CHARACTER_COUNT: i32 = 7;
const DYNAMIC_ISLAND_TIMER_CHARACTER_WIDTH: i32 = 8;
const DYNAMIC_ISLAND_TRAY_GAP_HEIGHT: i32 = 8;
const DYNAMIC_ISLAND_TRAY_PADDING_HEIGHT: i32 = 24;

pub(crate) fn default_overlay_levels() -> Vec<f32> {
    vec![0.0; LIVE_METER_BAR_COUNT]
}

fn hanning_window(index: usize, length: usize) -> f32 {
    if length <= 1 {
        return 1.0;
    }

    let phase = (2.0 * std::f32::consts::PI * index as f32) / (length - 1) as f32;
    0.5 - 0.5 * phase.cos()
}

fn goertzel_power(samples: &[f32], sample_rate: u32, target_frequency: f32) -> f32 {
    if samples.is_empty() || sample_rate == 0 {
        return 0.0;
    }

    let normalized_frequency = (target_frequency / sample_rate as f32).clamp(0.0, 0.5);
    if normalized_frequency <= 0.0 {
        return 0.0;
    }

    let omega = 2.0 * std::f32::consts::PI * normalized_frequency;
    let coefficient = 2.0 * omega.cos();
    let mut q1 = 0.0f32;
    let mut q2 = 0.0f32;

    for (index, sample) in samples.iter().enumerate() {
        let weighted = *sample * hanning_window(index, samples.len());
        let q0 = coefficient * q1 - q2 + weighted;
        q2 = q1;
        q1 = q0;
    }

    q1 * q1 + q2 * q2 - coefficient * q1 * q2
}

pub(crate) fn measure_overlay_levels(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() || sample_rate == 0 {
        return default_overlay_levels();
    }

    let analysis_len = samples.len().min(LIVE_METER_ANALYSIS_SAMPLES).max(256);
    let window = &samples[samples.len().saturating_sub(analysis_len)..];
    let (sum_squares, peak) = window.iter().fold((0.0f32, 0.0f32), |(sum, peak), sample| {
        let magnitude = sample.abs();
        (sum + sample * sample, peak.max(magnitude))
    });
    let rms = (sum_squares / window.len() as f32).sqrt();

    if rms < LIVE_METER_SILENCE_RMS_THRESHOLD && peak < LIVE_METER_SILENCE_PEAK_THRESHOLD {
        return default_overlay_levels();
    }

    let nyquist = sample_rate as f32 * 0.5;
    let min_frequency = MIN_ANALYSIS_FREQ_HZ;
    let max_frequency = (nyquist * NYQUIST_FRACTION)
        .min(MAX_ANALYSIS_FREQ_HZ)
        .max(min_frequency * 1.5);
    let ratio = (max_frequency / min_frequency).powf(1.0 / (LIVE_METER_BAR_COUNT as f32 - 1.0));

    let powers = (0..LIVE_METER_BAR_COUNT)
        .map(|index| {
            let center_frequency = min_frequency * ratio.powf(index as f32);
            goertzel_power(window, sample_rate, center_frequency)
        })
        .collect::<Vec<_>>();

    let max_power = powers.iter().copied().fold(0.0f32, f32::max).max(1e-9);
    let rms_drive = ((rms - LIVE_METER_SILENCE_RMS_THRESHOLD)
        / (LIVE_METER_FULL_RMS - LIVE_METER_SILENCE_RMS_THRESHOLD))
        .clamp(0.0, 1.0);
    let peak_drive = ((peak - LIVE_METER_SILENCE_PEAK_THRESHOLD)
        / (LIVE_METER_FULL_PEAK - LIVE_METER_SILENCE_PEAK_THRESHOLD))
        .clamp(0.0, 1.0);
    let activity = rms_drive.max(peak_drive).powf(ACTIVITY_EXPONENT);

    if activity <= ACTIVITY_GATE_THRESHOLD {
        return default_overlay_levels();
    }

    let mut levels = default_overlay_levels();
    for (index, level) in levels.iter_mut().enumerate() {
        let normalized = (powers[index] / max_power).clamp(0.0, 1.0).sqrt();
        let gated = (normalized * activity).clamp(0.0, 1.0);
        *level = if gated < LEVEL_GATE_THRESHOLD {
            0.0
        } else {
            gated
        };
    }

    levels
}

fn compact_indicator_row_width(style: &OverlayAnimationStyle, show_timer: bool) -> i32 {
    let timer_width = TIMER_CHARACTER_COUNT * TIMER_CHARACTER_WIDTH + TIMER_INSET_WIDTH;

    let signal_width = match style {
        OverlayAnimationStyle::Radial => RADIAL_SIGNAL_WIDTH,
        OverlayAnimationStyle::Spectrum => SPECTRUM_SIGNAL_WIDTH,
        OverlayAnimationStyle::Waveform => WAVEFORM_SIGNAL_WIDTH,
    };

    let base_width = if matches!(style, OverlayAnimationStyle::Radial) {
        signal_width
    } else {
        STATUS_WIDTH + GAP_WIDTH + signal_width
    };

    if show_timer {
        base_width + GAP_WIDTH + timer_width
    } else {
        base_width
    }
}

fn live_transcript_width_px(width: &LiveTranscriptWidth) -> i32 {
    match width {
        LiveTranscriptWidth::Compact => COMPACT_TRANSCRIPT_WIDTH,
        LiveTranscriptWidth::Balanced => BALANCED_TRANSCRIPT_WIDTH,
        LiveTranscriptWidth::Wide => WIDE_TRANSCRIPT_WIDTH,
    }
}

fn live_transcript_line_count(lines: &LiveTranscriptLines) -> i32 {
    match lines {
        LiveTranscriptLines::One => 1,
        LiveTranscriptLines::Two => 2,
        LiveTranscriptLines::Three => 3,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DynamicIslandLayout {
    left_pod_width: i32,
    right_pod_width: i32,
    gap_width: i32,
    row_height: i32,
    tray_height: i32,
}

impl DynamicIslandLayout {
    fn total_width(self) -> i32 {
        self.left_pod_width + self.gap_width + self.right_pod_width
    }

    fn total_height(self, show_live_transcription: bool) -> i32 {
        self.row_height
            + if show_live_transcription {
                DYNAMIC_ISLAND_TRAY_GAP_HEIGHT + self.tray_height
            } else {
                0
            }
    }
}

fn dynamic_island_layout(
    settings: &Settings,
    metrics: Option<crate::platform::DynamicIslandMetrics>,
) -> DynamicIslandLayout {
    let gap_width = metrics
        .map(|metrics| metrics.width)
        .unwrap_or(DEFAULT_DYNAMIC_ISLAND_GAP_WIDTH)
        .max(72);
    let row_height = metrics
        .map(|metrics| metrics.height)
        .unwrap_or(DEFAULT_DYNAMIC_ISLAND_GAP_HEIGHT)
        .max(DEFAULT_DYNAMIC_ISLAND_GAP_HEIGHT);
    let signal_width = match settings.overlay_animation_style {
        OverlayAnimationStyle::Radial => RADIAL_SIGNAL_WIDTH,
        OverlayAnimationStyle::Spectrum => SPECTRUM_SIGNAL_WIDTH,
        OverlayAnimationStyle::Waveform => WAVEFORM_SIGNAL_WIDTH,
    };
    let left_pod_width = if matches!(
        settings.overlay_animation_style,
        OverlayAnimationStyle::Radial
    ) {
        signal_width + DYNAMIC_ISLAND_POD_HORIZONTAL_PADDING
    } else {
        STATUS_WIDTH + GAP_WIDTH + signal_width + DYNAMIC_ISLAND_POD_HORIZONTAL_PADDING
    };
    let right_pod_width = if settings.show_recording_timer {
        DYNAMIC_ISLAND_TIMER_CHARACTER_COUNT * DYNAMIC_ISLAND_TIMER_CHARACTER_WIDTH
            + DYNAMIC_ISLAND_POD_HORIZONTAL_PADDING
    } else {
        0
    };
    let tray_height = live_transcript_line_count(&settings.live_transcript_lines)
        * TRANSCRIPT_LINE_HEIGHT
        + DYNAMIC_ISLAND_TRAY_PADDING_HEIGHT;

    DynamicIslandLayout {
        left_pod_width,
        right_pod_width,
        gap_width,
        row_height,
        tray_height,
    }
}

fn fallback_indicator_window_size_with_dynamic_island(
    settings: &Settings,
    metrics: Option<crate::platform::DynamicIslandMetrics>,
) -> (i32, i32) {
    if matches!(settings.overlay_position, OverlayPosition::DynamicIsland) {
        let layout = dynamic_island_layout(settings, metrics);
        return (
            layout.total_width() + INDICATOR_WINDOW_PADDING * 2,
            layout.total_height(settings.show_live_transcription) + INDICATOR_WINDOW_PADDING * 2,
        );
    }

    let content_height = if settings.show_live_transcription {
        let copy_height =
            live_transcript_line_count(&settings.live_transcript_lines) * TRANSCRIPT_LINE_HEIGHT;
        let row_height = if matches!(
            settings.overlay_animation_style,
            OverlayAnimationStyle::Radial
        ) {
            RADIAL_ROW_HEIGHT
        } else {
            NON_RADIAL_ROW_HEIGHT
        };
        copy_height + row_height + TRANSCRIPT_GAP_HEIGHT + SHELL_PADDING_HEIGHT
    } else {
        COMPACT_INDICATOR_HEIGHT
    };

    let pill_padding_width = if settings.show_live_transcription {
        PILL_PADDING_WITH_TRANSCRIPT
    } else {
        PILL_PADDING_WITHOUT_TRANSCRIPT
    };
    let content_width = if settings.show_live_transcription {
        let live_text_width = live_transcript_width_px(&settings.live_transcript_width);
        let row_width = compact_indicator_row_width(
            &settings.overlay_animation_style,
            settings.show_recording_timer,
        );
        live_text_width.max(row_width) + pill_padding_width
    } else {
        compact_indicator_row_width(
            &settings.overlay_animation_style,
            settings.show_recording_timer,
        ) + pill_padding_width
    };

    (
        content_width + INDICATOR_WINDOW_PADDING * 2,
        content_height + INDICATOR_WINDOW_PADDING * 2,
    )
}

fn dynamic_island_indicator_origin(
    settings: &Settings,
    metrics: crate::platform::DynamicIslandMetrics,
) -> PhysicalPosition<i32> {
    let layout = dynamic_island_layout(settings, Some(metrics));
    PhysicalPosition::new(
        metrics.x - layout.left_pod_width,
        metrics.y + DYNAMIC_ISLAND_MARGIN_TOP,
    )
}

pub(crate) fn fallback_indicator_window_size(settings: &Settings) -> (i32, i32) {
    fallback_indicator_window_size_with_dynamic_island(settings, None)
}

pub(crate) fn update_overlay_elapsed(core: &mut AppCore) {
    if let Some(started_at) = core.recording_started_at {
        core.overlay.elapsed_ms = started_at.elapsed().as_millis() as u64;
    }
}

pub(crate) fn clear_overlay_session_state(core: &mut AppCore) {
    core.overlay.visible = false;
    core.overlay.title.clear();
    core.overlay.detail.clear();
    core.overlay.levels = default_overlay_levels();
    core.overlay.elapsed_ms = 0;
    core.overlay.limit_ms = None;
    core.overlay.anchor = None;
    core.recording_started_at = None;
}

fn indicator_origin(
    app: &AppHandle,
    overlay: &OverlaySnapshot,
    settings: &Settings,
    indicator_width: i32,
    indicator_height: i32,
    dynamic_island_metrics: Option<crate::platform::DynamicIslandMetrics>,
) -> Option<PhysicalPosition<i32>> {
    let window = app.get_webview_window("indicator")?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let work_area = monitor.work_area();
    let left = work_area.position.x;
    let top = work_area.position.y;
    let width = work_area.size.width as i32;
    let height = work_area.size.height as i32;
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let monitor_left = monitor_position.x;
    let monitor_top = monitor_position.y;
    let monitor_width = monitor_size.width as i32;
    let work_max_x = left + width - indicator_width;
    let monitor_max_x = monitor_left + monitor_width - indicator_width;
    let position = settings.overlay_position;
    let dynamic_island_origin =
        dynamic_island_metrics.map(|metrics| dynamic_island_indicator_origin(settings, metrics));

    if matches!(position, OverlayPosition::Caret) {
        if let Some(anchor) = overlay.anchor {
            let min_x = left + INDICATOR_MARGIN;
            let max_x = left + width - indicator_width - INDICATOR_MARGIN;
            let min_y = top + INDICATOR_MARGIN;
            let max_y = top + height - indicator_height - INDICATOR_MARGIN;

            return Some(PhysicalPosition::new(
                anchor.x.clamp(min_x, max_x.max(min_x)),
                anchor.y.clamp(min_y, max_y.max(min_y)),
            ));
        }
    }

    let x = match position {
        OverlayPosition::DynamicIsland => dynamic_island_origin
            .as_ref()
            .map(|origin| origin.x)
            .unwrap_or_else(|| left + (width - indicator_width) / 2),
        OverlayPosition::TopLeft | OverlayPosition::BottomLeft => left + INDICATOR_MARGIN,
        OverlayPosition::TopRight | OverlayPosition::BottomRight => {
            left + width - indicator_width - INDICATOR_MARGIN
        }
        OverlayPosition::TopCenter | OverlayPosition::BottomCenter | OverlayPosition::Caret => {
            left + (width - indicator_width) / 2
        }
    };
    let y = match position {
        OverlayPosition::DynamicIsland => dynamic_island_origin
            .as_ref()
            .map(|origin| origin.y)
            .unwrap_or(top + INDICATOR_MARGIN),
        OverlayPosition::TopLeft | OverlayPosition::TopCenter | OverlayPosition::TopRight => {
            top + INDICATOR_MARGIN
        }
        OverlayPosition::BottomLeft
        | OverlayPosition::BottomRight
        | OverlayPosition::BottomCenter
        | OverlayPosition::Caret => top + height - indicator_height - INDICATOR_MARGIN,
    };
    let (min_x, max_x, min_y) = match position {
        OverlayPosition::DynamicIsland => (
            monitor_left,
            monitor_max_x.max(monitor_left),
            dynamic_island_metrics
                .map(|metrics| metrics.y)
                .unwrap_or(monitor_top),
        ),
        _ => (left, work_max_x.max(left), top),
    };

    Some(PhysicalPosition::new(x.clamp(min_x, max_x), y.max(min_y)))
}

pub(crate) fn update_indicator_window(app: &AppHandle, shared: &SharedState) {
    let (overlay, settings, measured_size) = {
        let core = shared.lock();
        (
            core.overlay.clone(),
            core.settings.clone(),
            core.indicator_window_size,
        )
    };

    let Some(window) = app.get_webview_window("indicator") else {
        return;
    };

    let dynamic_island_metrics =
        if matches!(settings.overlay_position, OverlayPosition::DynamicIsland) {
            crate::platform::dynamic_island_metrics(app)
        } else {
            None
        };
    let (indicator_width, indicator_height) = measured_size.unwrap_or_else(|| {
        fallback_indicator_window_size_with_dynamic_island(&settings, dynamic_island_metrics)
    });
    let _ = window.set_size(Size::Physical(PhysicalSize::new(
        indicator_width.max(1) as u32,
        indicator_height.max(1) as u32,
    )));

    if overlay.visible {
        if let Some(position) = indicator_origin(
            app,
            &overlay,
            &settings,
            indicator_width,
            indicator_height,
            dynamic_island_metrics,
        ) {
            let _ = window.set_position(Position::Physical(position));
        }
        let _ = window.show();
    } else {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dynamic_island_indicator_origin, fallback_indicator_window_size,
        fallback_indicator_window_size_with_dynamic_island, measure_overlay_levels,
    };
    use crate::platform::DynamicIslandMetrics;
    use crate::state::{OverlayAnimationStyle, OverlayPosition, Settings};

    #[test]
    fn measure_overlay_levels_stays_still_for_silence() {
        let silence = vec![0.0f32; 2_048];
        let levels = measure_overlay_levels(&silence, 16_000);

        assert!(levels.iter().all(|level| *level == 0.0));
    }

    #[test]
    fn measure_overlay_levels_reacts_to_spoken_energy() {
        let voiced = (0..2_048)
            .map(|index| {
                let phase = 2.0 * std::f32::consts::PI * 220.0 * index as f32 / 16_000.0;
                phase.sin() * 0.08
            })
            .collect::<Vec<_>>();
        let levels = measure_overlay_levels(&voiced, 16_000);

        assert!(levels.iter().any(|level| *level > 0.05));
    }

    fn generate_sine(freq: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f32) as usize;
        (0..num_samples)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    #[test]
    fn measure_overlay_levels_returns_correct_bar_count() {
        use crate::constants::LIVE_METER_BAR_COUNT;
        let sine = generate_sine(440.0, 0.2, 16_000);
        let levels = measure_overlay_levels(&sine, 16_000);
        assert_eq!(levels.len(), LIVE_METER_BAR_COUNT);
    }

    #[test]
    fn measure_overlay_levels_empty_samples_all_zeros() {
        let levels = measure_overlay_levels(&[], 16_000);
        assert!(levels.iter().all(|l| *l == 0.0));
    }

    #[test]
    fn measure_overlay_levels_values_in_unit_range() {
        let loud_sine = generate_sine(1000.0, 0.2, 16_000)
            .iter()
            .map(|s| s * 0.5)
            .collect::<Vec<_>>();
        let levels = measure_overlay_levels(&loud_sine, 16_000);
        for level in &levels {
            assert!(
                *level >= 0.0 && *level <= 1.0,
                "level out of range: {level}"
            );
        }
    }

    #[test]
    fn low_freq_concentrated_in_lower_bars() {
        let low = generate_sine(120.0, 0.3, 16_000)
            .iter()
            .map(|s| s * 0.15)
            .collect::<Vec<_>>();
        let levels = measure_overlay_levels(&low, 16_000);
        let lower_half: f32 = levels[..levels.len() / 2].iter().sum();
        let upper_half: f32 = levels[levels.len() / 2..].iter().sum();
        assert!(
            lower_half >= upper_half,
            "low freq should have more energy in lower bars: lower={lower_half} upper={upper_half}"
        );
    }

    #[test]
    fn high_freq_concentrated_in_upper_bars() {
        let high = generate_sine(4000.0, 0.3, 16_000)
            .iter()
            .map(|s| s * 0.15)
            .collect::<Vec<_>>();
        let levels = measure_overlay_levels(&high, 16_000);
        let lower_half: f32 = levels[..levels.len() / 2].iter().sum();
        let upper_half: f32 = levels[levels.len() / 2..].iter().sum();
        assert!(
            upper_half >= lower_half,
            "high freq should have more energy in upper bars: lower={lower_half} upper={upper_half}"
        );
    }

    #[test]
    fn default_overlay_levels_has_correct_length() {
        use crate::constants::LIVE_METER_BAR_COUNT;
        let levels = super::default_overlay_levels();
        assert_eq!(levels.len(), LIVE_METER_BAR_COUNT);
        assert!(levels.iter().all(|l| *l == 0.0));
    }

    #[test]
    fn dynamic_island_width_ignores_transcript_width_presets() {
        let mut compact = Settings::default();
        compact.overlay_position = OverlayPosition::DynamicIsland;
        compact.overlay_animation_style = OverlayAnimationStyle::Spectrum;
        compact.show_recording_timer = true;
        compact.show_live_transcription = true;

        let mut wide = compact.clone();
        compact.live_transcript_width = crate::state::LiveTranscriptWidth::Compact;
        wide.live_transcript_width = crate::state::LiveTranscriptWidth::Wide;

        let metrics = Some(DynamicIslandMetrics {
            x: 600,
            y: 0,
            width: 118,
            height: 32,
        });
        assert_eq!(
            fallback_indicator_window_size_with_dynamic_island(&compact, metrics),
            fallback_indicator_window_size_with_dynamic_island(&wide, metrics)
        );
    }

    #[test]
    fn dynamic_island_origin_aligns_left_pod_to_detected_gap() {
        let mut settings = Settings::default();
        settings.overlay_position = OverlayPosition::DynamicIsland;
        settings.overlay_animation_style = OverlayAnimationStyle::Spectrum;
        settings.show_recording_timer = true;

        let metrics = DynamicIslandMetrics {
            x: 640,
            y: 4,
            width: 118,
            height: 32,
        };

        let origin = dynamic_island_indicator_origin(&settings, metrics);

        assert_eq!(origin.x, 524);
        assert_eq!(origin.y, 4);
    }

    #[test]
    fn non_dynamic_island_preserves_transcript_width_presets() {
        let mut compact = Settings::default();
        compact.overlay_position = OverlayPosition::TopCenter;
        compact.show_live_transcription = true;

        let mut wide = compact.clone();
        compact.live_transcript_width = crate::state::LiveTranscriptWidth::Compact;
        wide.live_transcript_width = crate::state::LiveTranscriptWidth::Wide;

        assert_ne!(
            fallback_indicator_window_size(&compact),
            fallback_indicator_window_size(&wide)
        );
    }
}
