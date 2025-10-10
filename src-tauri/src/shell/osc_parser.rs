use percent_encoding::percent_decode_str;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum ShellIntegrationState {
    Disabled,
    Detecting,
    Enabled,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CommandStatus {
    Ready,
    Running,
    Finished { exit_code: Option<i32> },
}

#[derive(Debug, Clone)]
pub enum OscSequence {
    CurrentWorkingDirectory {
        path: String,
    },
    WindowsTerminalCwd {
        path: String,
    },
    ShellIntegration {
        marker: IntegrationMarker,
        data: Option<String>,
    },
    WindowTitle {
        title_type: WindowTitleType,
        title: String,
    },
    /// OrbitX 自定义协议: Node 版本同步
    OrbitXNodeVersion {
        version: String,
    },
    Unknown {
        command: String,
        params: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntegrationMarker {
    PromptStart,
    CommandStart,
    CommandExecuted,
    CommandFinished { exit_code: Option<i32> },
    CommandContinuation,
    RightPrompt,
    CommandInvalid,
    CommandCancelled,
    Property { key: String, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowTitleType {
    Both,
    Icon,
    Window,
}

pub struct OscParser;

impl OscParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, data: &str) -> Vec<OscSequence> {
        let mut sequences = Vec::new();
        let bytes = data.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() {
            if let Some((start, end, term_len)) = find_sequence(bytes, idx) {
                if let Some(seq) = self.parse_payload(&data[start + 2..end - term_len]) {
                    sequences.push(seq);
                }
                idx = end;
            } else {
                idx += 1;
            }
        }
        sequences
    }

    pub fn strip_osc_sequences(&self, data: &str) -> String {
        let bytes = data.as_bytes();
        let mut idx = 0;
        let mut last = 0;
        let mut result = String::with_capacity(data.len());
        while idx < bytes.len() {
            if let Some((start, end, _)) = find_sequence(bytes, idx) {
                if start > last {
                    result.push_str(&data[last..start]);
                }
                idx = end;
                last = end;
            } else {
                idx += 1;
            }
        }
        if last < data.len() {
            result.push_str(&data[last..]);
        }
        result
    }

    fn parse_payload(&self, payload: &str) -> Option<OscSequence> {
        let (cmd, rest) = payload.split_once(';')?;
        match cmd {
            "7" => parse_cwd(rest).map(|path| OscSequence::CurrentWorkingDirectory { path }),
            "9" => parse_windows_cwd(rest).map(|path| OscSequence::WindowsTerminalCwd { path }),
            "0" => Some(OscSequence::WindowTitle {
                title_type: WindowTitleType::Both,
                title: rest.to_string(),
            }),
            "1" => Some(OscSequence::WindowTitle {
                title_type: WindowTitleType::Icon,
                title: rest.to_string(),
            }),
            "2" => Some(OscSequence::WindowTitle {
                title_type: WindowTitleType::Window,
                title: rest.to_string(),
            }),
            "133" => parse_shell_integration(rest),
            "1337" => parse_orbitx_custom(rest),
            _ => Some(OscSequence::Unknown {
                command: cmd.to_string(),
                params: rest.to_string(),
            }),
        }
    }
}

fn find_sequence(bytes: &[u8], start: usize) -> Option<(usize, usize, usize)> {
    if bytes.get(start)? != &0x1b || bytes.get(start + 1)? != &b']' {
        return None;
    }
    let mut idx = start + 2;
    while idx < bytes.len() {
        match bytes[idx] {
            0x07 => return Some((start, idx + 1, 1)),
            0x1b if idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' => {
                return Some((start, idx + 2, 2))
            }
            _ => idx += 1,
        }
    }
    None
}

fn parse_cwd(data: &str) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let value = data.strip_prefix("file://").unwrap_or(data);
    let path = match value.find('/') {
        Some(pos) => &value[pos..],
        None => value,
    };
    percent_decode_str(path)
        .decode_utf8()
        .ok()
        .map(Cow::into_owned)
}

fn parse_windows_cwd(data: &str) -> Option<String> {
    let mut parts = data.splitn(2, ';');
    match (parts.next(), parts.next()) {
        (Some("9"), Some(path)) => Some(path.to_string()),
        (Some("9"), None) => Some(String::new()),
        _ => None,
    }
}

fn parse_shell_integration(data: &str) -> Option<OscSequence> {
    let marker = data.chars().next()?;
    let rest = data[marker.len_utf8()..].trim_start_matches(';');
    let marker = match marker {
        'A' | 'a' => IntegrationMarker::PromptStart,
        'B' | 'b' => IntegrationMarker::CommandStart,
        'C' | 'c' => IntegrationMarker::CommandExecuted,
        'D' | 'd' => IntegrationMarker::CommandFinished {
            exit_code: parse_exit_code(rest),
        },
        'E' | 'e' => IntegrationMarker::CommandContinuation,
        'F' | 'f' => IntegrationMarker::RightPrompt,
        'G' | 'g' => IntegrationMarker::CommandInvalid,
        'H' | 'h' => IntegrationMarker::CommandCancelled,
        'P' | 'p' => {
            let (key, value) = rest.split_once('=')?;
            IntegrationMarker::Property {
                key: key.to_string(),
                value: value.to_string(),
            }
        }
        _ => return None,
    };

    let payload = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    Some(OscSequence::ShellIntegration {
        marker,
        data: payload,
    })
}

/// 解析 OrbitX 自定义协议
fn parse_orbitx_custom(data: &str) -> Option<OscSequence> {
    if let Some(version) = data.strip_prefix("OrbitXNodeVersion=") {
        Some(OscSequence::OrbitXNodeVersion {
            version: version.to_string(),
        })
    } else {
        Some(OscSequence::Unknown {
            command: "1337".to_string(),
            params: data.to_string(),
        })
    }
}

fn parse_exit_code(data: &str) -> Option<i32> {
    if data.is_empty() {
        return None;
    }
    if let Ok(code) = data.parse::<i32>() {
        return Some(code);
    }
    for token in data.split(|c: char| c == ';' || c == '=' || c.is_whitespace()) {
        if let Ok(code) = token.parse::<i32>() {
            return Some(code);
        }
    }
    None
}
