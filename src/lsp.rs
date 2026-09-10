use std::{
    collections::HashSet,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub severity: u64,
    pub message: String,
}

#[derive(Debug)]
pub enum LspEvent {
    Diagnostics {
        path: PathBuf,
        items: Vec<Diagnostic>,
    },
    Completion {
        id: u64,
        items: Vec<String>,
    },
    Message(String),
}

pub struct LspClient {
    child: Child,
    input: Arc<Mutex<ChildStdin>>,
    events: Receiver<LspEvent>,
    opened: HashSet<PathBuf>,
    next_id: u64,
    name: String,
}

impl LspClient {
    pub fn start(root: &Path, command: &[String]) -> io::Result<Self> {
        let (program, arguments) = command
            .split_first()
            .ok_or_else(|| io::Error::other("empty LSP command"))?;
        let mut child = Command::new(program)
            .args(arguments)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = Arc::new(Mutex::new(
            child
                .stdin
                .take()
                .ok_or_else(|| io::Error::other("LSP stdin unavailable"))?,
        ));
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("LSP stdout unavailable"))?;
        let (sender, events) = mpsc::channel();
        let response_input = Arc::clone(&input);
        thread::spawn(move || read_messages(stdout, response_input, sender));

        let mut client = Self {
            child,
            input,
            events,
            opened: HashSet::new(),
            next_id: 2,
            name: program.clone(),
        };
        client.write(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": path_uri(root),
                "capabilities": {
                    "textDocument": {
                        "completion": {"completionItem": {"snippetSupport": false}},
                        "publishDiagnostics": {"relatedInformation": true}
                    }
                },
                "clientInfo": {"name": "TIDE", "version": env!("CARGO_PKG_VERSION")}
            }
        }))?;
        client.notify("initialized", json!({}))?;
        Ok(client)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn open_or_change(&mut self, path: &Path, text: &str, version: i32) -> io::Result<()> {
        let path = path.to_path_buf();
        let uri = path_uri(&path);
        if self.opened.insert(path.clone()) {
            self.notify(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": language_id(&path),
                        "version": version,
                        "text": text
                    }
                }),
            )
        } else {
            self.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [{"text": text}]
                }),
            )
        }
    }

    pub fn close(&mut self, path: &Path) -> io::Result<()> {
        if self.opened.remove(path) {
            self.notify(
                "textDocument/didClose",
                json!({
                    "textDocument": {"uri": path_uri(path)}
                }),
            )?;
        }
        Ok(())
    }

    pub fn completion(&mut self, path: &Path, line: usize, column: usize) -> io::Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": path_uri(path)},
                "position": {"line": line, "character": column},
                "context": {"triggerKind": 1}
            }
        }))?;
        Ok(id)
    }

    pub fn try_event(&self) -> Option<LspEvent> {
        self.events.try_recv().ok()
    }

    fn notify(&mut self, method: &str, params: Value) -> io::Result<()> {
        self.write(json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    fn write(&mut self, message: Value) -> io::Result<()> {
        let body = serde_json::to_vec(&message).map_err(io::Error::other)?;
        let mut input = self
            .input
            .lock()
            .map_err(|_| io::Error::other("LSP input lock poisoned"))?;
        write!(input, "Content-Length: {}\r\n\r\n", body.len())?;
        input.write_all(&body)?;
        input.flush()
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.write(
            json!({"jsonrpc": "2.0", "id": self.next_id, "method": "shutdown", "params": null}),
        );
        let _ = self.notify("exit", json!(null));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_messages(stdout: impl Read, input: Arc<Mutex<ChildStdin>>, sender: mpsc::Sender<LspEvent>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut length = None;
        loop {
            let mut header = String::new();
            if reader
                .read_line(&mut header)
                .ok()
                .filter(|value| *value > 0)
                .is_none()
            {
                return;
            }
            if header == "\r\n" || header == "\n" {
                break;
            }
            if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().ok();
            }
        }
        let Some(length) = length else { continue };
        let mut body = vec![0; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };
        if value.get("method").is_some() && value.get("id").is_some() {
            let result =
                if value.get("method").and_then(Value::as_str) == Some("workspace/configuration") {
                    let count = value
                        .pointer("/params/items")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len);
                    Value::Array(vec![Value::Null; count])
                } else {
                    Value::Null
                };
            if let Some(id) = value.get("id") {
                let response = json!({"jsonrpc": "2.0", "id": id, "result": result});
                if let Ok(body) = serde_json::to_vec(&response)
                    && let Ok(mut stream) = input.lock()
                {
                    let _ = write!(stream, "Content-Length: {}\r\n\r\n", body.len());
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                }
            }
        } else if value.get("method").and_then(Value::as_str)
            == Some("textDocument/publishDiagnostics")
        {
            let Some(uri) = value.pointer("/params/uri").and_then(Value::as_str) else {
                continue;
            };
            let items = value
                .pointer("/params/diagnostics")
                .and_then(Value::as_array)
                .map(|values| values.iter().filter_map(parse_diagnostic).collect())
                .unwrap_or_default();
            let _ = sender.send(LspEvent::Diagnostics {
                path: uri_path(uri),
                items,
            });
        } else if let Some(id) = value.get("id").and_then(Value::as_u64)
            && let Some(result) = value.get("result")
        {
            let values = result
                .as_array()
                .or_else(|| result.get("items").and_then(Value::as_array));
            if let Some(values) = values {
                let items = values
                    .iter()
                    .filter_map(|item| {
                        item.get("insertText")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("label").and_then(Value::as_str))
                            .map(str::to_owned)
                    })
                    .take(100)
                    .collect();
                let _ = sender.send(LspEvent::Completion { id, items });
            }
        } else if let Some(message) = value.pointer("/params/message").and_then(Value::as_str) {
            let _ = sender.send(LspEvent::Message(message.to_owned()));
        }
    }
}

fn parse_diagnostic(value: &Value) -> Option<Diagnostic> {
    Some(Diagnostic {
        line: value.pointer("/range/start/line")?.as_u64()? as usize,
        column: value.pointer("/range/start/character")?.as_u64()? as usize,
        severity: value.get("severity").and_then(Value::as_u64).unwrap_or(3),
        message: value.get("message")?.as_str()?.replace('\n', " "),
    })
}

fn path_uri(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    let escaped = value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<String>();
    if escaped.starts_with('/') {
        format!("file://{escaped}")
    } else {
        format!("file:///{escaped}")
    }
}

fn uri_path(uri: &str) -> PathBuf {
    let value = uri.strip_prefix("file://").unwrap_or(uri);
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%'
            && index + 2 < raw.len()
            && let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16)
        {
            bytes.push(byte);
            index += 3;
        } else {
            bytes.push(raw[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8_lossy(&bytes).into_owned();
    #[cfg(windows)]
    let decoded = decoded.strip_prefix('/').unwrap_or(&decoded).to_owned();
    PathBuf::from(decoded)
}

fn language_id(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
    {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascriptreact",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "hpp" => "cpp",
        "json" => "json",
        "html" => "html",
        "css" => "css",
        "sh" | "bash" => "shellscript",
        _ => "plaintext",
    }
}

pub fn detect_command(root: &Path) -> Option<Vec<String>> {
    let candidates: &[(&str, &[&str], &str)] = &[
        ("Cargo.toml", &["rust-analyzer"], "rust-analyzer"),
        ("go.mod", &["gopls"], "gopls"),
        ("pyproject.toml", &["pylsp"], "pylsp"),
        (
            "package.json",
            &["typescript-language-server", "--stdio"],
            "typescript-language-server",
        ),
    ];
    for (marker, command, executable) in candidates {
        if root.join(marker).exists() && command_exists(executable) {
            return Some(command.iter().map(|value| (*value).to_owned()).collect());
        }
    }
    None
}

fn command_exists(command: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|path| {
        let candidate = path.join(command);
        candidate.is_file() || cfg!(windows) && candidate.with_extension("exe").is_file()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_round_trips_spaces_and_unicode() {
        let path = if cfg!(windows) {
            PathBuf::from("C:/code/tide café/main.rs")
        } else {
            PathBuf::from("/tmp/tide café/main.rs")
        };
        assert_eq!(uri_path(&path_uri(&path)), path);
    }

    #[test]
    fn maps_common_language_ids() {
        assert_eq!(language_id(Path::new("main.rs")), "rust");
        assert_eq!(language_id(Path::new("app.tsx")), "typescriptreact");
        assert_eq!(language_id(Path::new("tool.py")), "python");
    }
}
