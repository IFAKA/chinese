use std::process::Command;

pub fn speak(text: &str) {
    if std::env::var("CHINESE_AUDIO").as_deref() == Ok("off") {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        // Eddy is a locally installed mainland-Mandarin male voice on modern
        // macOS. Allow an override for Macs with a different installed voice.
        let voice = std::env::var("CHINESE_VOICE")
            .unwrap_or_else(|_| "Eddy (Chinese (China mainland))".to_owned());
        let status = Command::new("say").args(["-v", &voice, text]).status();
        if status.is_err() || !status.is_ok_and(|value| value.success()) {
            let _ = Command::new("say").args(["-v", "Tingting", text]).status();
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = text;
}
