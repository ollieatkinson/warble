use anyhow::Result;
use tauri::AppHandle;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneAccess {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
}

pub(crate) fn ensure_microphone_access(app: &AppHandle) -> Result<MicrophoneAccess> {
    imp::ensure_microphone_access(app)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn ensure_post_event_access(app: &AppHandle) -> Result<()> {
    imp::ensure_post_event_access(app)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn request_post_event_access(app: &AppHandle) -> Result<bool> {
    imp::request_post_event_access(app)
}

pub(crate) fn microphone_access_status_message(state: MicrophoneAccess) -> &'static str {
    match state {
        MicrophoneAccess::Restricted => "Microphone access restricted",
        MicrophoneAccess::Denied => "Microphone access denied",
        MicrophoneAccess::NotDetermined => "Microphone access required",
        MicrophoneAccess::Authorized => "Ready",
    }
}

pub(crate) fn microphone_access_error_message(state: MicrophoneAccess) -> &'static str {
    match state {
        MicrophoneAccess::Restricted => {
            "macOS has restricted microphone access for Warble. Check your device restrictions or Screen Time settings."
        }
        MicrophoneAccess::Denied => {
            "Warble needs microphone access on macOS. Enable it in System Settings > Privacy & Security > Microphone."
        }
        MicrophoneAccess::NotDetermined => {
            "Warble needs microphone access on macOS before it can list or record from your inputs."
        }
        MicrophoneAccess::Authorized => "Microphone access is available.",
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn post_event_access_error_message() -> &'static str {
    "Warble needs Accessibility permission on macOS to paste into other apps. Enable Warble in System Settings > Privacy & Security > Accessibility."
}

#[cfg(target_os = "macos")]
mod imp {
    use anyhow::{anyhow, bail, Context, Result};
    use block2::RcBlock;
    use chrono::Utc;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
    use objc2_core_graphics::CGRequestPostEventAccess;
    use std::sync::mpsc;
    use tauri::AppHandle;

    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    use crate::storage::append_capture_log;

    use super::{post_event_access_error_message, MicrophoneAccess};

    pub(super) fn ensure_microphone_access(app: &AppHandle) -> Result<MicrophoneAccess> {
        let state = current_microphone_access()?;
        log_microphone_access(app, "status", format!("{state:?}"));

        match state {
            MicrophoneAccess::Authorized => Ok(MicrophoneAccess::Authorized),
            MicrophoneAccess::NotDetermined => request_microphone_access(app),
            state => Ok(state),
        }
    }

    pub(super) fn ensure_post_event_access(app: &AppHandle) -> Result<()> {
        let granted = request_post_event_access(app)?;
        if granted {
            Ok(())
        } else {
            bail!(post_event_access_error_message())
        }
    }

    pub(super) fn request_post_event_access(app: &AppHandle) -> Result<bool> {
        // AXIsProcessTrusted checks the Accessibility TCC entry directly and
        // is reliable for ad-hoc signed apps. CGPreflightPostEventAccess can
        // return false even when the Accessibility toggle is enabled.
        if unsafe { AXIsProcessTrusted() } {
            return Ok(true);
        }

        run_on_main_thread_and_wait(app, || CGRequestPostEventAccess())
    }

    fn current_microphone_access() -> Result<MicrophoneAccess> {
        let media_type = audio_media_type()?;
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };

        Ok(match status {
            AVAuthorizationStatus::NotDetermined => MicrophoneAccess::NotDetermined,
            AVAuthorizationStatus::Restricted => MicrophoneAccess::Restricted,
            AVAuthorizationStatus::Denied => MicrophoneAccess::Denied,
            AVAuthorizationStatus::Authorized => MicrophoneAccess::Authorized,
            _ => MicrophoneAccess::Denied,
        })
    }

    fn request_microphone_access(app: &AppHandle) -> Result<MicrophoneAccess> {
        let (sender, receiver) = mpsc::channel::<Result<bool>>();
        log_microphone_access(app, "request-started", "using AVCaptureDevice");

        run_on_main_thread_and_wait(app, move || {
            let completion_sender = sender.clone();
            let handler = RcBlock::new(move |granted: Bool| {
                let _ = completion_sender.send(Ok(granted.as_bool()));
            });

            let media_type =
                unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio should be available on macOS");
            unsafe {
                AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
            }
        })?;

        let granted = receiver
            .recv()
            .context("macOS microphone permission request did not complete")??;
        let state = if granted {
            MicrophoneAccess::Authorized
        } else {
            current_microphone_access()?
        };
        log_microphone_access(
            app,
            "request-finished",
            format!("granted={granted} state={state:?}"),
        );

        Ok(state)
    }

    fn audio_media_type() -> Result<&'static objc2_av_foundation::AVMediaType> {
        unsafe { AVMediaTypeAudio }
            .ok_or_else(|| anyhow!("AVMediaTypeAudio is unavailable on this macOS runtime"))
    }

    fn log_microphone_access(app: &AppHandle, stage: &str, detail: impl Into<String>) {
        let detail = detail.into();
        let message = if detail.is_empty() {
            format!(
                "{} [permissions] microphone {stage}",
                Utc::now().to_rfc3339()
            )
        } else {
            format!(
                "{} [permissions] microphone {stage}: {detail}",
                Utc::now().to_rfc3339()
            )
        };
        append_capture_log(app, &message);
    }

    fn run_on_main_thread_and_wait<T, F>(app: &AppHandle, task: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        if unsafe { libc::pthread_main_np() == 1 } {
            return Ok(task());
        }

        let (sender, receiver) = mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = sender.send(task());
        })
        .context("failed to run macOS permission request on the main thread")?;

        receiver
            .recv()
            .context("macOS main-thread permission request did not complete")
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use anyhow::Result;
    use tauri::AppHandle;

    use super::MicrophoneAccess;

    pub(super) fn ensure_microphone_access(_app: &AppHandle) -> Result<MicrophoneAccess> {
        Ok(MicrophoneAccess::Authorized)
    }

    #[allow(dead_code)]
    pub(super) fn ensure_post_event_access(_app: &AppHandle) -> Result<()> {
        Ok(())
    }

    pub(super) fn request_post_event_access(_app: &AppHandle) -> Result<bool> {
        Ok(true)
    }
}
