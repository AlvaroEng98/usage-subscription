use serde_json::Value;
use std::{collections::HashSet, fs::File, io::{self, BufRead, BufReader}, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub source: &'static str,
    pub project: Option<String>,
    pub session_id: String,
    pub message_id: Option<String>,
    pub recorded_at: Option<String>,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub source_file: PathBuf,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScanSummary {
    pub files: usize,
    pub malformed_lines: usize,
    pub events: Vec<UsageEvent>,
}

pub fn scan_projects(root: &Path) -> io::Result<ScanSummary> {
    let mut summary = ScanSummary::default();
    let mut files = Vec::new();
    collect_jsonl_files(root, &mut files)?;

    for file in files {
        summary.files += 1;
        scan_transcript(root, &file, &mut summary)?;
    }

    Ok(summary)
}

fn collect_jsonl_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    Ok(())
}

fn scan_transcript(root: &Path, path: &Path, summary: &mut ScanSummary) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let session_id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let mut seen_message_ids = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        let value = match serde_json::from_str::<Value>(&line) {
            Ok(value) => value,
            Err(_) => {
                summary.malformed_lines += 1;
                continue;
            }
        };

        if let Some(message_id) = value
            .get("message")
            .and_then(|message| message.get("id"))
            .and_then(Value::as_str)
            && !seen_message_ids.insert(message_id.to_owned())
        {
            continue;
        }

        if let Some(event) = usage_event(&value, root, &session_id, path) {
            summary.events.push(event);
        }
    }

    Ok(())
}

fn usage_event(value: &Value, root: &Path, session_id: &str, source_file: &Path) -> Option<UsageEvent> {
    if value.get("type")?.as_str()? != "assistant" {
        return None;
    }

    let message = value.get("message")?;
    let usage = message.get("usage")?;
    let model = message.get("model").and_then(Value::as_str).map(str::to_owned);
    let project = source_file
        .strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .map(|component| component.as_os_str().to_string_lossy().into_owned());

    Some(UsageEvent {
        source: "claude-code",
        project,
        session_id: session_id.to_owned(),
        message_id: message.get("id").and_then(Value::as_str).map(str::to_owned),
        recorded_at: value.get("timestamp").and_then(Value::as_str).map(str::to_owned),
        model,
        input_tokens: number(usage, "input_tokens"),
        output_tokens: number(usage, "output_tokens"),
        cache_read_tokens: number(usage, "cache_read_input_tokens"),
        cache_creation_tokens: number(usage, "cache_creation_input_tokens"),
        source_file: source_file.to_owned(),
    })
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scans_usage_from_assistant_record() {
        let root = std::env::temp_dir().join(format!("usage-subscription-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("session.jsonl");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, r#"{{"type":"user","message":{{"content":"hello"}}}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg-1","model":"claude-sonnet","usage":{{"input_tokens":10,"output_tokens":4,"cache_read_input_tokens":2,"cache_creation_input_tokens":3}}}}}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","message":{{"id":"msg-1","model":"claude-sonnet","usage":{{"input_tokens":10,"output_tokens":4}}}}}}"#).unwrap();

        let summary = scan_projects(&root).unwrap();
        assert_eq!(summary.files, 1);
        assert_eq!(summary.malformed_lines, 0);
        assert_eq!(summary.events.len(), 1);
        assert_eq!(summary.events[0].input_tokens, 10);
        assert_eq!(summary.events[0].cache_creation_tokens, 3);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_records_without_usage() {
        let value = serde_json::json!({
            "type": "assistant",
            "message": { "model": "claude-sonnet" }
        });
        assert!(usage_event(&value, Path::new("."), "session", Path::new("session.jsonl")).is_none());
    }
}
