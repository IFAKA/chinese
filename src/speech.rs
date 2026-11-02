use std::{
    fs, io,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub const SILENCE_SECONDS: f32 = 1.5;
pub const MAX_CAPTURE_SECONDS: f32 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechEvent {
    Partial(String),
    Final(String),
    Unavailable(String),
}

pub struct SpeechSession {
    child: Child,
    output_path: PathBuf,
    offset: usize,
}

impl SpeechSession {
    pub fn start() -> io::Result<Self> {
        let helper = option_env!("CHINESE_SPEECH_HELPER").ok_or_else(|| {
            io::Error::new(io::ErrorKind::Unsupported, "speech helper is unavailable")
        })?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let output_path = std::env::temp_dir().join(format!(
            "chinese-speech-{}-{stamp}.jsonl",
            std::process::id()
        ));
        let child = Command::new(helper)
            .arg(&output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            output_path,
            offset: 0,
        })
    }

    pub fn poll(&mut self) -> io::Result<Vec<SpeechEvent>> {
        let contents = match fs::read_to_string(&self.output_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let new = &contents[self.offset.min(contents.len())..];
        self.offset = contents.len();
        Ok(new.lines().filter_map(parse_event).collect())
    }

    pub fn finished(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_some()
    }
}

impl Drop for SpeechSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = fs::remove_file(&self.output_path);
    }
}

fn parse_event(line: &str) -> Option<SpeechEvent> {
    let mut parts = line.splitn(2, '\t');
    match parts.next()? {
        "partial" => Some(SpeechEvent::Partial(parts.next()?.to_owned())),
        "final" => Some(SpeechEvent::Final(
            parts.next().unwrap_or_default().to_owned(),
        )),
        "unavailable" => Some(SpeechEvent::Unavailable(
            parts.next().unwrap_or_default().to_owned(),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_partial_final_and_unavailable_events() {
        assert_eq!(
            parse_event("partial\t你好"),
            Some(SpeechEvent::Partial("你好".into()))
        );
        assert_eq!(
            parse_event("final\t你好"),
            Some(SpeechEvent::Final("你好".into()))
        );
        assert_eq!(
            parse_event("unavailable\tpermission denied"),
            Some(SpeechEvent::Unavailable("permission denied".into()))
        );
        assert_eq!(parse_event("unknown\tvalue"), None);
    }
}
