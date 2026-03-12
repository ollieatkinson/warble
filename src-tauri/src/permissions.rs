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

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn resolve_microphone_access<F>(
    state: MicrophoneAccess,
    request_access: F,
) -> Result<MicrophoneAccess>
where
    F: FnOnce() -> Result<MicrophoneAccess>,
{
    match state {
        MicrophoneAccess::Authorized => Ok(MicrophoneAccess::Authorized),
        MicrophoneAccess::NotDetermined => request_access(),
        state => Ok(state),
    }
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
        super::resolve_microphone_access(current_microphone_access()?, request_microphone_access)
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

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};

    use super::{
        microphone_access_error_message, microphone_access_status_message,
        resolve_microphone_access, MicrophoneAccess,
    };

    #[test]
    fn resolve_microphone_access_skips_request_when_already_authorized() {
        let mut requested = false;

        let resolved = resolve_microphone_access(MicrophoneAccess::Authorized, || {
            requested = true;
            Ok(MicrophoneAccess::Denied)
        })
        .expect("authorized access should succeed");

        assert_eq!(resolved, MicrophoneAccess::Authorized);
        assert!(!requested);
    }

    #[test]
    fn resolve_microphone_access_preserves_denied_state_without_prompting() {
        let mut requested = false;

        let resolved = resolve_microphone_access(MicrophoneAccess::Denied, || {
            requested = true;
            Ok(MicrophoneAccess::Authorized)
        })
        .expect("denied access should be returned as-is");

        assert_eq!(resolved, MicrophoneAccess::Denied);
        assert!(!requested);
    }

    #[test]
    fn resolve_microphone_access_requests_when_status_is_not_determined() {
        let mut requested = false;

        let resolved = resolve_microphone_access(MicrophoneAccess::NotDetermined, || {
            requested = true;
            Ok(MicrophoneAccess::Authorized)
        })
        .expect("request result should be returned");

        assert_eq!(resolved, MicrophoneAccess::Authorized);
        assert!(requested);
    }

    #[test]
    fn resolve_microphone_access_propagates_request_failures() {
        let error = resolve_microphone_access(MicrophoneAccess::NotDetermined, || -> Result<_> {
            Err(anyhow!("permission request failed"))
        })
        .expect_err("request failure should bubble up");

        assert_eq!(error.to_string(), "permission request failed");
    }

    #[test]
    fn microphone_access_messages_use_warble_branding() {
        for state in [
            MicrophoneAccess::NotDetermined,
            MicrophoneAccess::Restricted,
            MicrophoneAccess::Denied,
        ] {
            assert!(
                microphone_access_error_message(state).contains("Warble"),
                "expected {state:?} guidance to mention Warble"
            );
        }

        assert_eq!(
            microphone_access_status_message(MicrophoneAccess::Restricted),
            "Microphone access restricted"
        );
        assert_eq!(
            microphone_access_status_message(MicrophoneAccess::Denied),
            "Microphone access denied"
        );
        assert_eq!(
            microphone_access_status_message(MicrophoneAccess::NotDetermined),
            "Microphone access required"
        );
    }
}
