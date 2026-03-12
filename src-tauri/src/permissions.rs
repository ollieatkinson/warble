use anyhow::Result;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneAccess {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
}

pub(crate) fn ensure_microphone_access() -> Result<MicrophoneAccess> {
    imp::ensure_microphone_access()
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
            "macOS has restricted microphone access for Transcribed. Check your device restrictions or Screen Time settings."
        }
        MicrophoneAccess::Denied => {
            "Transcribed needs microphone access on macOS. Enable it in System Settings > Privacy & Security > Microphone."
        }
        MicrophoneAccess::NotDetermined => {
            "Transcribed needs microphone access on macOS before it can list or record from your inputs."
        }
        MicrophoneAccess::Authorized => "Microphone access is available.",
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use anyhow::{anyhow, Context, Result};
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{
        AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeAudio,
    };
    use std::sync::mpsc;

    use super::MicrophoneAccess;

    pub(super) fn ensure_microphone_access() -> Result<MicrophoneAccess> {
        match current_microphone_access()? {
            MicrophoneAccess::Authorized => Ok(MicrophoneAccess::Authorized),
            MicrophoneAccess::NotDetermined => request_microphone_access(),
            state => Ok(state),
        }
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

    fn request_microphone_access() -> Result<MicrophoneAccess> {
        let media_type = audio_media_type()?;
        let (sender, receiver) = mpsc::channel();
        let handler = RcBlock::new(move |granted: Bool| {
            let _ = sender.send(granted.as_bool());
        });

        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
        }

        let granted = receiver
            .recv()
            .context("macOS microphone permission request did not complete")?;

        Ok(if granted {
            MicrophoneAccess::Authorized
        } else {
            MicrophoneAccess::Denied
        })
    }

    fn audio_media_type() -> Result<&'static AVMediaType> {
        unsafe { AVMediaTypeAudio.ok_or_else(|| anyhow!("macOS audio media type is unavailable")) }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use anyhow::Result;

    use super::MicrophoneAccess;

    pub(super) fn ensure_microphone_access() -> Result<MicrophoneAccess> {
        Ok(MicrophoneAccess::Authorized)
    }
}
