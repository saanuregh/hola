mod app;
mod helper;
use app::*;
use pamsm::{pam_module, Pam, PamError, PamFlag, PamLibExt, PamMsgStyle, PamServiceModule};
use std::{
    path::Path,
    time::{Duration, Instant},
};

struct PamTime;

impl PamServiceModule for PamTime {
    fn authenticate(pamh: Pam, _flags: PamFlag, _args: Vec<String>) -> PamError {
        authenticate(pamh)
    }

    fn open_session(pamh: Pam, _flags: PamFlag, _args: Vec<String>) -> PamError {
        authenticate(pamh)
    }

    fn close_session(_pamh: Pam, _flags: PamFlag, _args: Vec<String>) -> PamError {
        PamError::SUCCESS
    }

    fn setcred(_pamh: Pam, _flags: PamFlag, _args: Vec<String>) -> PamError {
        PamError::SUCCESS
    }
}

fn authenticate(pamh: Pam) -> PamError {
    let base_path = Path::new("/lib/security/pam_hola");
    let user = match pamh.get_user(None) {
        Ok(Some(u)) => {
            if let Ok(u_str) = u.to_str() {
                u_str
            } else {
                return PamError::USER_UNKNOWN;
            }
        }
        Ok(None) => return PamError::USER_UNKNOWN,
        Err(e) => return e,
    };
    let a = &mut App::new(base_path, user);

    // Abort is Hola is disabled
    if a.config().core.disabled {
        return PamError::AUTHINFO_UNAVAIL;
    }

    // Abort if we're in a remote SSH env
    if a.config().core.ignore_ssh {
        let keys = vec!["SSH_CONNECTION", "SSH_CLIENT", "SSHD_OPTS"];
        if keys.iter().any(|k| std::env::var(k).is_ok()) {
            return PamError::AUTHINFO_UNAVAIL;
        }
    }

    // Abort if lid is closed
    if a.config().core.ignore_closed_lid {
        let output = std::process::Command::new("cat")
            .arg("/proc/acpi/button/lid/*/state")
            .output()
            .unwrap();
        if String::from_utf8(output.stdout).unwrap().contains("closed") {
            return PamError::AUTHINFO_UNAVAIL;
        }
    }

    // Alert the user that we are doing face detection
    if a.config().core.detection_notice {
        pamh.conv(Some("Attempting face detection"), PamMsgStyle::TEXT_INFO)
            .unwrap();
    }

    // Couldn't find any face model for the user
    if a.models().is_empty() {
        if !a.config().core.suppress_unknown {
            pamh.conv(Some("No face model known"), PamMsgStyle::ERROR_MSG)
                .unwrap();
        }
        return PamError::USER_UNKNOWN;
    }

    // Detection loop
    a.start_capture();
    let timeout = Duration::from_secs(a.config().video.timeout);
    let start_time = Instant::now();
    while start_time.elapsed() <= timeout {
        if let Some(encodings) = a.process_next_frame() {
            if encodings.iter().any(|e| a.identify(e.clone())) {
                if !a.config().core.no_confirmation {
                    pamh.conv(
                        Some(&format!(
                            "Identified face as {} in {:?}",
                            user,
                            start_time.elapsed()
                        )),
                        PamMsgStyle::TEXT_INFO,
                    )
                    .unwrap();
                }
                return PamError::SUCCESS;
            }
        }
    }

    // Timeout reached
    if !a.config().core.suppress_timeout {
        pamh.conv(
            Some("Face detection timeout reached"),
            PamMsgStyle::ERROR_MSG,
        )
        .unwrap();
    }
    PamError::AUTH_ERR
}

pam_module!(PamTime);
