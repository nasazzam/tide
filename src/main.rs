use std::{
    collections::{HashMap, HashSet},
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
    },
};
use ratatui_image::{
    Resize, StatefulImage,
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
};
use unicode_width::UnicodeWidthStr;

mod config;
mod lsp;

use config::TideConfig;
use lsp::{Diagnostic, LspClient, LspEvent};

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Explorer,
    Editor,
}

#[derive(Clone, PartialEq, Eq)]
struct TreeRow {
    path: PathBuf,
    depth: usize,
    is_dir: bool,
}

type Document = (
    String,
    Vec<String>,
    bool,
    Option<DynamicImage>,
    SystemTime,
    u64,
);

fn read_document(path: &Path) -> io::Result<Document> {
    const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
    const MAX_BINARY_PREVIEW: usize = 1024 * 1024;

    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(io::Error::other(
            "file is larger than the 50 MiB preview limit",
        ));
    }
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let bytes = fs::read(path)?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let identity = format!("{}:{:x}", bytes.len(), hasher.finish());

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let visual = if extension == "pdf" {
        render_pdf_page(path).ok()
    } else {
        image::load_from_memory(&bytes)
            .ok()
            .or_else(|| render_yazi_style_preview(path, &extension).ok())
    };
    if let Some(image) = visual {
        return Ok((
            identity,
            vec![format!(
                "Visual preview: {} × {}",
                image.width(),
                image.height()
            )],
            true,
            Some(image),
            modified,
            metadata.len(),
        ));
    }

    match String::from_utf8(bytes) {
        Ok(text) => {
            let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
            if lines.is_empty() {
                lines.push(String::new());
            }
            Ok((text, lines, false, None, modified, metadata.len()))
        }
        Err(error) => {
            let bytes = error.into_bytes();
            let mut lines = Vec::new();
            for (offset, chunk) in bytes.chunks(16).take(MAX_BINARY_PREVIEW / 16).enumerate() {
                let hex = chunk
                    .iter()
                    .map(|b| format!("{b:02x} "))
                    .collect::<String>();
                let ascii = chunk
                    .iter()
                    .map(|b| {
                        if b.is_ascii_graphic() || *b == b' ' {
                            *b as char
                        } else {
                            '·'
                        }
                    })
                    .collect::<String>();
                lines.push(format!("{:08x}  {:<48} │{}│", offset * 16, hex, ascii));
            }
            if bytes.len() > MAX_BINARY_PREVIEW {
                lines.push(format!(
                    "… preview limited to 1 MiB of {} bytes",
                    bytes.len()
                ));
            }
            if lines.is_empty() {
                lines.push("Empty binary file".into());
            }
            Ok((identity, lines, true, None, modified, metadata.len()))
        }
    }
}

fn preview_prefix(path: &Path, kind: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    env::temp_dir().join(format!(
        "tide-editor-{kind}-{}-{:x}",
        std::process::id(),
        hasher.finish()
    ))
}

fn render_pdf_page(path: &Path) -> io::Result<DynamicImage> {
    // Match Yazi's PDF pipeline: Poppler renders one page to a quality-controlled
    // JPEG, then the terminal image driver performs the final fit.
    let prefix = preview_prefix(path, "pdf");
    let output = Command::new("pdftoppm")
        .args([
            "-f",
            "1",
            "-l",
            "1",
            "-singlefile",
            "-jpeg",
            "-jpegopt",
            "quality=85",
        ])
        .arg(path)
        .arg(&prefix)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let jpeg = prefix.with_extension("jpg");
    let image = image::open(&jpeg).map_err(io::Error::other)?;
    let _ = fs::remove_file(jpeg);
    Ok(image)
}

fn render_yazi_style_preview(path: &Path, extension: &str) -> io::Result<DynamicImage> {
    let videos = ["mp4", "mkv", "webm", "avi", "mov", "m4v", "flv", "wmv"];
    let output_path = preview_prefix(path, "media").with_extension("png");
    let output = if videos.contains(&extension) {
        Command::new("ffmpegthumbnailer")
            .args(["-i"])
            .arg(path)
            .args(["-o"])
            .arg(&output_path)
            .args(["-s", "0", "-q", "8"])
            .output()?
    } else {
        // Yazi also delegates formats such as SVG, AVIF, HEIC, JXL, and fonts
        // to ImageMagick when its built-in decoder cannot handle them.
        Command::new("magick")
            .arg(path)
            .args(["-thumbnail", "1600x1600>"])
            .arg(&output_path)
            .output()?
    };
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let image = image::open(&output_path).map_err(io::Error::other)?;
    let _ = fs::remove_file(output_path);
    Ok(image)
}

struct Editor {
    path: PathBuf,
    lines: Vec<String>,
    row: usize,
    col: usize,
    scroll_y: usize,
    scroll_x: usize,
    dirty: bool,
    disk_text: String,
    externally_changed: HashSet<usize>,
    conflict: bool,
    read_only: bool,
    visual: Option<DynamicImage>,
    visual_protocol: Option<StatefulProtocol>,
    disk_modified: SystemTime,
    disk_size: u64,
    undo: Vec<Vec<String>>,
    redo: Vec<Vec<String>>,
    lsp_sent_text: String,
    lsp_version: i32,
}

impl Editor {
    fn open(path: PathBuf) -> io::Result<Self> {
        let (content, lines, read_only, visual, disk_modified, disk_size) = read_document(&path)?;
        Ok(Self {
            path,
            lines,
            row: 0,
            col: 0,
            scroll_y: 0,
            scroll_x: 0,
            dirty: false,
            disk_text: content,
            externally_changed: HashSet::new(),
            conflict: false,
            read_only,
            visual,
            visual_protocol: None,
            disk_modified,
            disk_size,
            undo: Vec::new(),
            redo: Vec::new(),
            lsp_sent_text: String::new(),
            lsp_version: 1,
        })
    }

    fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    fn line_len(&self) -> usize {
        self.lines[self.row].chars().count()
    }

    fn tab_label(&self) -> String {
        let state = if self.conflict {
            "⚠"
        } else if self.dirty {
            "●"
        } else if !self.externally_changed.is_empty() {
            "↻"
        } else if self.read_only {
            "◇"
        } else {
            ""
        };
        if state.is_empty() {
            format!(" {}  × ", self.name())
        } else {
            format!(" {state} {}  × ", self.name())
        }
    }

    fn snapshot(&mut self) {
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(self.lines.clone());
        self.redo.clear();
        self.dirty = true;
    }

    fn insert(&mut self, ch: char) {
        if self.read_only {
            return;
        }
        self.snapshot();
        let idx = byte_index(&self.lines[self.row], self.col);
        self.lines[self.row].insert(idx, ch);
        self.col += 1;
    }

    fn insert_text(&mut self, text: &str) {
        if self.read_only || text.is_empty() {
            return;
        }
        self.snapshot();
        for ch in text.chars() {
            if ch == '\n' {
                self.newline_raw();
            } else if ch != '\r' {
                let idx = byte_index(&self.lines[self.row], self.col);
                self.lines[self.row].insert(idx, ch);
                self.col += 1;
            }
        }
    }

    fn newline_raw(&mut self) {
        let idx = byte_index(&self.lines[self.row], self.col);
        let tail = self.lines[self.row].split_off(idx);
        self.row += 1;
        self.lines.insert(self.row, tail);
        self.col = 0;
    }

    fn newline(&mut self) {
        if self.read_only {
            return;
        }
        self.snapshot();
        self.newline_raw();
    }

    fn backspace(&mut self) {
        if self.read_only {
            return;
        }
        if self.col > 0 {
            self.snapshot();
            let start = byte_index(&self.lines[self.row], self.col - 1);
            let end = byte_index(&self.lines[self.row], self.col);
            self.lines[self.row].replace_range(start..end, "");
            self.col -= 1;
        } else if self.row > 0 {
            self.snapshot();
            let current = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.lines[self.row].chars().count();
            self.lines[self.row].push_str(&current);
        }
    }

    fn delete(&mut self) {
        if self.read_only {
            return;
        }
        if self.col < self.line_len() {
            self.snapshot();
            let start = byte_index(&self.lines[self.row], self.col);
            let end = byte_index(&self.lines[self.row], self.col + 1);
            self.lines[self.row].replace_range(start..end, "");
        } else if self.row + 1 < self.lines.len() {
            self.snapshot();
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.lines.clone());
            self.lines = previous;
            self.clamp_cursor();
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.lines.clone());
            self.lines = next;
            self.clamp_cursor();
            self.dirty = true;
        }
    }

    fn clamp_cursor(&mut self) {
        self.row = self.row.min(self.lines.len().saturating_sub(1));
        self.col = self.col.min(self.line_len());
    }

    fn save(&mut self) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::other("binary preview is read-only"));
        }
        if self.conflict {
            return Err(io::Error::other(
                "file changed on disk; press Ctrl+R to reload before saving",
            ));
        }
        let text = self.lines.join("\n");
        fs::write(&self.path, &text)?;
        self.disk_text = text;
        if let Ok(metadata) = fs::metadata(&self.path) {
            self.disk_modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            self.disk_size = metadata.len();
        }
        self.dirty = false;
        self.externally_changed.clear();
        Ok(())
    }

    fn reload_from_disk(&mut self) -> io::Result<bool> {
        let (text, new_lines, read_only, visual, disk_modified, disk_size) =
            read_document(&self.path)?;
        if text == self.disk_text && !self.conflict {
            return Ok(false);
        }
        let old_lines = self.lines.clone();
        self.externally_changed = changed_line_numbers(&old_lines, &new_lines);
        self.lines = new_lines;
        self.disk_text = text;
        self.dirty = false;
        self.conflict = false;
        self.read_only = read_only;
        self.visual = visual;
        self.visual_protocol = None;
        self.disk_modified = disk_modified;
        self.disk_size = disk_size;
        self.undo.clear();
        self.redo.clear();
        self.clamp_cursor();
        Ok(true)
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct DiffSummary {
    files: usize,
    hunks: usize,
    additions: usize,
    deletions: usize,
}

impl DiffSummary {
    fn label(&self) -> String {
        if self.files == 0 {
            " Δ DIFF · CLEAN ".into()
        } else {
            format!(
                " Δ DIFF · {}F {}H +{} -{} ",
                self.files, self.hunks, self.additions, self.deletions
            )
        }
    }
}

#[derive(Clone)]
struct SearchResult {
    path: PathBuf,
    line: Option<usize>,
    label: String,
}

struct QuickOpen {
    query: String,
    files: Vec<PathBuf>,
    results: Vec<SearchResult>,
    selected: usize,
}

#[derive(Clone, Copy)]
enum FileAction {
    CreateFile,
    CreateDirectory,
    Rename,
    Move,
    Delete,
}

struct FilePrompt {
    action: FileAction,
    input: String,
    target: Option<PathBuf>,
}

struct CompletionMenu {
    items: Vec<String>,
    selected: usize,
}

struct App {
    root: PathBuf,
    expanded: HashSet<PathBuf>,
    tree_cache: Vec<TreeRow>,
    last_tree_refresh: Instant,
    selected: Option<PathBuf>,
    show_hidden: bool,
    explorer_visible: bool,
    explorer_scroll_x: usize,
    tabs: Vec<Editor>,
    active: usize,
    focus: Focus,
    message: String,
    quit: bool,
    last_watch: Instant,
    quick_open: Option<QuickOpen>,
    syntax_set: SyntaxSet,
    syntax_theme: Theme,
    git_status: HashMap<PathBuf, char>,
    git_enabled: bool,
    last_git_refresh: Instant,
    diff_summary: DiffSummary,
    launch_hunk_foreground: bool,
    image_picker: Picker,
    image_protocol_name: String,
    explorer_width: u16,
    prompt: Option<FilePrompt>,
    completion: Option<CompletionMenu>,
    pending_completion: Option<u64>,
    diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
    lsp: Option<LspClient>,
}

impl App {
    fn new(root: PathBuf, image_picker: Picker, config: TideConfig) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        let rows = tree_rows(&root, &expanded, config.editor.show_hidden);
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let themes = ThemeSet::load_defaults();
        let git_enabled = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .is_ok_and(|output| output.status.success());
        let syntax_theme = themes.themes["base16-ocean.dark"].clone();
        let image_protocol_name = format!("{:?}", image_picker.protocol_type()).to_uppercase();
        let lsp_command = if config.lsp.disabled {
            None
        } else if config.lsp.command.is_empty() {
            lsp::detect_command(&root)
        } else {
            Some(config.lsp.command.clone())
        };
        let lsp = lsp_command
            .as_deref()
            .and_then(|command| LspClient::start(&root, command).ok());
        let lsp_message = lsp
            .as_ref()
            .map(|client| format!(" · LSP {}", client.name()))
            .unwrap_or_default();
        let mut app = Self {
            root,
            expanded,
            tree_cache: rows.clone(),
            last_tree_refresh: Instant::now(),
            selected: rows.first().map(|r| r.path.clone()),
            show_hidden: config.editor.show_hidden,
            explorer_visible: true,
            explorer_scroll_x: 0,
            tabs: Vec::new(),
            active: 0,
            focus: Focus::Explorer,
            message: format!(
                "Mouse · Ctrl+Shift+A/S/D/F/G views · Ctrl+E switch · Ctrl+S save · Ctrl+Q quit{lsp_message}"
            ),
            quit: false,
            last_watch: Instant::now(),
            quick_open: None,
            syntax_set,
            syntax_theme,
            git_status: HashMap::new(),
            git_enabled,
            last_git_refresh: Instant::now() - Duration::from_secs(10),
            diff_summary: DiffSummary::default(),
            launch_hunk_foreground: false,
            image_picker,
            image_protocol_name,
            explorer_width: config.editor.explorer_width,
            prompt: None,
            completion: None,
            pending_completion: None,
            diagnostics: HashMap::new(),
            lsp,
        };
        app.refresh_git_status();
        app.refresh_diff_summary();
        app
    }

    fn rows(&self) -> Vec<TreeRow> {
        self.tree_cache.clone()
    }

    fn refresh_tree(&mut self) -> bool {
        self.last_tree_refresh = Instant::now();
        let rows = tree_rows(&self.root, &self.expanded, self.show_hidden);
        if rows == self.tree_cache {
            return false;
        }
        self.tree_cache = rows;
        if self
            .selected
            .as_ref()
            .is_none_or(|selected| !self.tree_cache.iter().any(|row| &row.path == selected))
        {
            self.selected = self.tree_cache.first().map(|row| row.path.clone());
        }
        true
    }

    fn move_selection(&mut self, delta: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|p| rows.iter().position(|r| &r.path == p))
            .unwrap_or(0);
        let next = (current as isize + delta).clamp(0, rows.len() as isize - 1) as usize;
        self.selected = Some(rows[next].path.clone());
    }

    fn activate_selection(&mut self) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        if path.is_dir() {
            if !self.expanded.remove(&path) {
                self.expanded.insert(path);
            }
            self.refresh_tree();
        } else {
            self.open_file(path);
        }
    }

    fn collapse_or_parent(&mut self) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        if path.is_dir() && self.expanded.remove(&path) {
            self.refresh_tree();
            return;
        }
        let Some(parent) = path.parent() else { return };
        if parent.starts_with(&self.root) && parent != self.root {
            self.selected = Some(parent.to_path_buf());
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        if let Some(index) = self.tabs.iter().position(|e| e.path == path) {
            self.active = index;
            self.focus = Focus::Editor;
            return;
        }
        match Editor::open(path) {
            Ok(mut editor) => {
                if let Some(image) = editor.visual.as_ref() {
                    editor.visual_protocol =
                        Some(self.image_picker.new_resize_protocol(image.clone()));
                }
                self.tabs.push(editor);
                self.active = self.tabs.len() - 1;
                self.focus = Focus::Editor;
                self.message =
                    "Opened file · Ctrl+S saves · Ctrl+W closes · Ctrl+Space completes".into();
                self.sync_active_lsp();
            }
            Err(error) => self.message = format!("Cannot open: {error}"),
        }
    }

    fn save(&mut self) {
        let Some(editor) = self.tabs.get_mut(self.active) else {
            return;
        };
        match editor.save() {
            Ok(()) => self.message = format!("Saved {}", editor.path.display()),
            Err(error) => self.message = format!("Save failed: {error}"),
        }
    }

    fn close_tab(&mut self) {
        self.close_tab_at(self.active);
    }

    fn close_tab_at(&mut self, index: usize) {
        let Some(editor) = self.tabs.get(index) else {
            return;
        };
        if editor.dirty {
            self.active = index;
            self.message = "Unsaved file: save with Ctrl+S before closing".into();
            return;
        }
        let closed_path = self.tabs[index].path.clone();
        if let Some(lsp) = self.lsp.as_mut() {
            let _ = lsp.close(&closed_path);
        }
        self.diagnostics.remove(&closed_path);
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = 0;
            self.focus = Focus::Explorer;
        } else if index < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.tabs.len() - 1);
        }
    }

    fn reload_active(&mut self) {
        let Some(editor) = self.tabs.get_mut(self.active) else {
            return;
        };
        match editor.reload_from_disk() {
            Ok(true) => {
                if let Some(image) = editor.visual.as_ref() {
                    editor.visual_protocol =
                        Some(self.image_picker.new_resize_protocol(image.clone()));
                }
                self.message = format!("Reloaded {} from disk", editor.name());
            }
            Ok(false) => self.message = "File already matches disk".into(),
            Err(error) => self.message = format!("Reload failed: {error}"),
        }
    }

    fn refresh_git_status(&mut self) -> bool {
        self.last_git_refresh = Instant::now();
        if !self.git_enabled {
            return false;
        }
        let Ok(output) = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .output()
        else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let mut next_status = HashMap::new();
        for record in output.stdout.split(|byte| *byte == 0) {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            let status = &record[..2];
            let marker = if status.contains(&b'?') {
                '?'
            } else if status.contains(&b'D') {
                'D'
            } else if status.contains(&b'A') {
                'A'
            } else {
                'M'
            };
            let relative = String::from_utf8_lossy(&record[3..]);
            let path = self.root.join(relative.as_ref());
            next_status.insert(path.clone(), marker);
            let mut parent = path.parent();
            while let Some(directory) = parent {
                if directory == self.root || !directory.starts_with(&self.root) {
                    break;
                }
                next_status.entry(directory.to_path_buf()).or_insert(marker);
                parent = directory.parent();
            }
        }
        if self.git_status == next_status {
            false
        } else {
            self.git_status = next_status;
            true
        }
    }

    fn refresh_diff_summary(&mut self) -> bool {
        if !self.git_enabled {
            return false;
        }

        let status = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .output();
        let diff = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["diff", "--no-ext-diff", "--unified=0", "HEAD", "--"])
            .output();
        let Ok(status) = status else {
            return false;
        };
        if !status.status.success() {
            return false;
        }

        let mut next = DiffSummary::default();
        for record in status.stdout.split(|byte| *byte == 0) {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            next.files += 1;
            if &record[..2] == b"??" {
                next.hunks += 1;
                let relative = String::from_utf8_lossy(&record[3..]);
                if let Ok(contents) = fs::read_to_string(self.root.join(relative.as_ref())) {
                    next.additions += contents.lines().count().max(1);
                }
            }
        }
        if let Ok(diff) = diff
            && diff.status.success()
        {
            for line in String::from_utf8_lossy(&diff.stdout).lines() {
                if line.starts_with("@@") {
                    next.hunks += 1;
                } else if line.starts_with('+') && !line.starts_with("+++") {
                    next.additions += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    next.deletions += 1;
                }
            }
        }

        if self.diff_summary == next {
            false
        } else {
            self.diff_summary = next;
            true
        }
    }

    fn open_hunk(&mut self) {
        if !self.git_enabled {
            self.message = "Diff review requires a Git workspace".into();
            return;
        }
        if !Command::new("hunk")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            self.message = "Hunk is not installed; see https://hunk.dev/docs/start/install/".into();
            return;
        }

        if env::var_os("TMUX").is_some() {
            match Command::new("tmux")
                .args(["new-window", "-n", "diff", "-c"])
                .arg(&self.root)
                .arg("exec hunk diff --watch")
                .status()
            {
                Ok(status) if status.success() => {
                    self.message = "Opened live Hunk review in tmux window: diff".into();
                }
                Ok(status) => self.message = format!("Could not open Hunk (tmux status {status})"),
                Err(error) => self.message = format!("Could not open Hunk: {error}"),
            }
        } else {
            self.launch_hunk_foreground = true;
        }
    }

    fn watch_open_files(&mut self) -> bool {
        let mut changed = self.poll_lsp();
        if self.last_tree_refresh.elapsed() >= Duration::from_secs(2) {
            changed |= self.refresh_tree();
        }
        if self.last_git_refresh.elapsed() >= Duration::from_secs(5) {
            changed |= self.refresh_git_status();
            changed |= self.refresh_diff_summary();
        }
        if self.last_watch.elapsed() < Duration::from_millis(600) {
            return changed;
        }
        self.last_watch = Instant::now();
        let mut reloaded = Vec::new();
        let mut conflicted = Vec::new();
        let image_picker = &mut self.image_picker;
        for editor in &mut self.tabs {
            let Ok(metadata) = fs::metadata(&editor.path) else {
                continue;
            };
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if modified == editor.disk_modified && metadata.len() == editor.disk_size {
                continue;
            }
            if editor.dirty {
                if !editor.conflict {
                    conflicted.push(editor.name());
                }
                editor.conflict = true;
            } else if editor.reload_from_disk().unwrap_or(false) {
                if let Some(image) = editor.visual.as_ref() {
                    editor.visual_protocol = Some(image_picker.new_resize_protocol(image.clone()));
                }
                reloaded.push(editor.name());
            }
        }
        if !conflicted.is_empty() {
            self.message = format!(
                "Disk conflict in {}; Ctrl+R reloads disk version",
                conflicted.join(", ")
            );
            changed = true;
        } else if !reloaded.is_empty() {
            self.message = format!("Live reloaded external changes: {}", reloaded.join(", "));
            changed = true;
        }
        changed
    }

    fn start_quick_open(&mut self) {
        self.quick_open = Some(QuickOpen {
            query: String::new(),
            files: project_files(&self.root, self.show_hidden),
            results: Vec::new(),
            selected: 0,
        });
        self.refresh_quick_open();
    }

    fn refresh_quick_open(&mut self) {
        let Some(palette) = self.quick_open.as_ref() else {
            return;
        };
        let query = palette.query.clone();
        let results = search_project(&self.root, &query, &palette.files);
        if let Some(palette) = self.quick_open.as_mut() {
            palette.results = results;
            palette.selected = palette
                .selected
                .min(palette.results.len().saturating_sub(1));
        }
    }

    fn accept_quick_open(&mut self) {
        let result = self
            .quick_open
            .as_ref()
            .and_then(|palette| palette.results.get(palette.selected).cloned());
        self.quick_open = None;
        let Some(result) = result else {
            return;
        };
        self.open_file(result.path);
        if let Some(line) = result.line
            && let Some(editor) = self.tabs.get_mut(self.active)
        {
            editor.row = line
                .saturating_sub(1)
                .min(editor.lines.len().saturating_sub(1));
            editor.col = 0;
        }
    }

    fn sync_active_lsp(&mut self) {
        let Some(editor) = self.tabs.get_mut(self.active) else {
            return;
        };
        let text = editor.lines.join("\n");
        if text == editor.lsp_sent_text {
            return;
        }
        editor.lsp_version += 1;
        if let Some(lsp) = self.lsp.as_mut()
            && lsp
                .open_or_change(&editor.path, &text, editor.lsp_version)
                .is_ok()
        {
            editor.lsp_sent_text = text;
        }
    }

    fn request_completion(&mut self) {
        self.sync_active_lsp();
        let Some(editor) = self.tabs.get(self.active) else {
            return;
        };
        let Some(lsp) = self.lsp.as_mut() else {
            self.message = "No language server configured; set lsp.command in .tide.toml".into();
            return;
        };
        let lsp_column: usize = editor.lines[editor.row]
            .chars()
            .take(editor.col)
            .map(char::len_utf16)
            .sum();
        match lsp.completion(&editor.path, editor.row, lsp_column) {
            Ok(id) => {
                self.pending_completion = Some(id);
                self.message = "Requesting LSP completions…".into();
            }
            Err(error) => self.message = format!("LSP completion failed: {error}"),
        }
    }

    fn poll_lsp(&mut self) -> bool {
        let mut changed = false;
        loop {
            let event = self.lsp.as_ref().and_then(LspClient::try_event);
            let Some(event) = event else { break };
            match event {
                LspEvent::Diagnostics { path, items } => {
                    self.diagnostics.insert(path, items);
                    changed = true;
                }
                LspEvent::Completion { id, items } if self.pending_completion == Some(id) => {
                    self.pending_completion = None;
                    if items.is_empty() {
                        self.message = "No LSP completions".into();
                    } else {
                        self.completion = Some(CompletionMenu { items, selected: 0 });
                    }
                    changed = true;
                }
                LspEvent::Completion { .. } => {}
                LspEvent::Message(message) => {
                    self.message = format!("LSP: {message}");
                    changed = true;
                }
            }
        }
        changed
    }

    fn start_file_action(&mut self, action: FileAction) {
        let target = self.selected.clone();
        let input = match action {
            FileAction::Rename => target
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_default(),
            FileAction::Move => target
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.root).ok())
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            FileAction::Delete => String::new(),
            _ => String::new(),
        };
        self.prompt = Some(FilePrompt {
            action,
            input,
            target,
        });
    }

    fn apply_file_action(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let value = prompt.input.trim();
        let result: io::Result<String> = (|| match prompt.action {
            FileAction::CreateFile | FileAction::CreateDirectory => {
                if value.is_empty() {
                    return Err(io::Error::other("path cannot be empty"));
                }
                let base = prompt
                    .target
                    .filter(|p| p.is_dir())
                    .unwrap_or_else(|| self.root.clone());
                let path = safe_project_path(&self.root, &base.join(value))?;
                if path.exists() {
                    return Err(io::Error::other("path already exists"));
                }
                if matches!(prompt.action, FileAction::CreateDirectory) {
                    fs::create_dir_all(&path)?;
                } else {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&path, "")?;
                }
                self.expanded.insert(base);
                self.selected = Some(path.clone());
                Ok(format!("Created {}", path.display()))
            }
            FileAction::Rename => {
                let source = prompt
                    .target
                    .ok_or_else(|| io::Error::other("nothing selected"))?;
                if value.is_empty() || value.contains('/') || value.contains('\\') {
                    return Err(io::Error::other("enter a single file or directory name"));
                }
                let destination = source.parent().unwrap_or(&self.root).join(value);
                self.move_path(&source, &destination)?;
                Ok(format!("Renamed to {value}"))
            }
            FileAction::Move => {
                let source = prompt
                    .target
                    .ok_or_else(|| io::Error::other("nothing selected"))?;
                if value.is_empty() {
                    return Err(io::Error::other("destination cannot be empty"));
                }
                let destination = safe_project_path(&self.root, &self.root.join(value))?;
                self.move_path(&source, &destination)?;
                Ok(format!("Moved to {}", destination.display()))
            }
            FileAction::Delete => {
                let target = prompt
                    .target
                    .ok_or_else(|| io::Error::other("nothing selected"))?;
                if target == self.root {
                    return Err(io::Error::other("the project root cannot be deleted"));
                }
                if value != "delete" {
                    return Err(io::Error::other(
                        "deletion cancelled; type delete to confirm",
                    ));
                }
                if self
                    .tabs
                    .iter()
                    .any(|editor| editor.path.starts_with(&target) && editor.dirty)
                {
                    return Err(io::Error::other("selected file has unsaved changes"));
                }
                if target.is_dir() {
                    fs::remove_dir_all(&target)?;
                } else {
                    fs::remove_file(&target)?;
                }
                let closed = self
                    .tabs
                    .iter()
                    .filter(|editor| editor.path.starts_with(&target))
                    .map(|editor| editor.path.clone())
                    .collect::<Vec<_>>();
                if let Some(lsp) = self.lsp.as_mut() {
                    for path in &closed {
                        let _ = lsp.close(path);
                    }
                }
                self.tabs.retain(|editor| !editor.path.starts_with(&target));
                self.diagnostics
                    .retain(|path, _| !path.starts_with(&target));
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
                self.selected = target.parent().map(Path::to_path_buf);
                Ok(format!("Deleted {}", target.display()))
            }
        })();
        self.message = result.unwrap_or_else(|error| format!("File operation failed: {error}"));
        self.refresh_tree();
        self.refresh_git_status();
    }

    fn move_path(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
        let destination = safe_project_path(&self.root, destination)?;
        if destination.exists() {
            return Err(io::Error::other("destination already exists"));
        }
        if source == self.root || destination.starts_with(source) {
            return Err(io::Error::other("invalid destination"));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(source, &destination)?;
        let moved = self
            .tabs
            .iter()
            .filter(|editor| editor.path.starts_with(source))
            .map(|editor| editor.path.clone())
            .collect::<Vec<_>>();
        if let Some(lsp) = self.lsp.as_mut() {
            for path in &moved {
                let _ = lsp.close(path);
            }
        }
        for editor in &mut self.tabs {
            if let Ok(suffix) = editor.path.strip_prefix(source) {
                editor.path = destination.join(suffix);
                editor.lsp_sent_text.clear();
            }
        }
        self.diagnostics.retain(|path, _| !path.starts_with(source));
        self.selected = Some(destination);
        Ok(())
    }

    fn request_quit(&mut self) {
        if self.tabs.iter().any(|e| e.dirty) {
            self.message = "Unsaved files remain; save or close them before quitting".into();
        } else {
            self.quit = true;
        }
    }
}

fn safe_project_path(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    use std::path::Component;
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Prefix(value) => normalized.push(value.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(io::Error::other("path escapes project"));
                }
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    if normalized == root || !normalized.starts_with(root) {
        return Err(io::Error::other("path must remain inside the project"));
    }
    let canonical_root = fs::canonicalize(root)?;
    let mut existing = normalized.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| io::Error::other("path has no existing parent"))?;
    }
    if !fs::canonicalize(existing)?.starts_with(canonical_root) {
        return Err(io::Error::other(
            "path traverses a symlink outside the project",
        ));
    }
    Ok(normalized)
}

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(value.len())
}

fn changed_line_numbers(old: &[String], new: &[String]) -> HashSet<usize> {
    let prefix = old
        .iter()
        .zip(new.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let end = new.len().saturating_sub(suffix);
    let mut changed: HashSet<usize> = (prefix..end).collect();
    if changed.is_empty() && old.len() != new.len() && !new.is_empty() {
        changed.insert(prefix.min(new.len() - 1));
    }
    changed
}

fn tree_rows(root: &Path, expanded: &HashSet<PathBuf>, show_hidden: bool) -> Vec<TreeRow> {
    fn visit(
        dir: &Path,
        depth: usize,
        expanded: &HashSet<PathBuf>,
        show_hidden: bool,
        out: &mut Vec<TreeRow>,
    ) {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = read_dir
            .filter_map(Result::ok)
            .filter(|entry| show_hidden || !entry.file_name().to_string_lossy().starts_with('.'))
            .collect();
        entries.sort_by_key(|entry| {
            (
                !entry.path().is_dir(),
                entry.file_name().to_string_lossy().to_lowercase(),
            )
        });
        for entry in entries {
            let path = entry.path();
            let is_dir = path.is_dir();
            out.push(TreeRow {
                path: path.clone(),
                depth,
                is_dir,
            });
            if is_dir && expanded.contains(&path) {
                visit(&path, depth + 1, expanded, show_hidden, out);
            }
        }
    }

    let mut rows = Vec::new();
    visit(root, 0, expanded, show_hidden, &mut rows);
    rows
}

fn project_files(root: &Path, show_hidden: bool) -> Vec<PathBuf> {
    fn visit(dir: &Path, show_hidden: bool, files: &mut Vec<PathBuf>) {
        if files.len() >= 20_000 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), ".git" | "node_modules" | "target" | ".cache") {
                continue;
            }
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                visit(&path, show_hidden, files);
            } else if kind.is_file() {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    visit(root, show_hidden, &mut files);
    files
}

fn fuzzy_score(candidate: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(candidate.len());
    }
    if let Some(position) = candidate.find(query) {
        return Some(position * 2 + candidate.len());
    }
    let mut position = 0;
    let mut score = candidate.len() * 2;
    for wanted in query.chars() {
        let found = candidate[position..].find(wanted)?;
        position += found + wanted.len_utf8();
        score += found;
    }
    Some(score)
}

fn search_project(root: &Path, query: &str, files: &[PathBuf]) -> Vec<SearchResult> {
    if let Some(content_query) = query.strip_prefix('%') {
        let needle = content_query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        for path in files {
            if results.len() >= 100 {
                break;
            }
            if fs::metadata(path).is_ok_and(|metadata| metadata.len() > 2 * 1024 * 1024) {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            for (line_index, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    let relative = path.strip_prefix(root).unwrap_or(path).display();
                    let excerpt: String = line.trim().chars().take(80).collect();
                    results.push(SearchResult {
                        path: path.clone(),
                        line: Some(line_index + 1),
                        label: format!("{relative}:{}  {excerpt}", line_index + 1),
                    });
                    if results.len() >= 100 {
                        break;
                    }
                }
            }
        }
        return results;
    }

    let needle = query.to_lowercase();
    let mut ranked: Vec<(usize, SearchResult)> = files
        .iter()
        .cloned()
        .filter_map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let score = fuzzy_score(&relative.to_lowercase(), &needle)?;
            Some((
                score,
                SearchResult {
                    path,
                    line: None,
                    label: relative,
                },
            ))
        })
        .collect();
    ranked.sort_by_key(|(score, result)| (*score, result.label.len()));
    ranked
        .into_iter()
        .take(100)
        .map(|(_, result)| result)
        .collect()
}

fn handle_prompt_key(app: &mut App, key: KeyEvent) -> bool {
    if app.prompt.is_none() {
        return false;
    }
    match key.code {
        KeyCode::Esc => app.prompt = None,
        KeyCode::Enter => app.apply_file_action(),
        KeyCode::Backspace => {
            app.prompt.as_mut().unwrap().input.pop();
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.prompt.as_mut().unwrap().input.clear()
        }
        KeyCode::Char(ch)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.prompt.as_mut().unwrap().input.push(ch)
        }
        _ => {}
    }
    true
}

fn handle_completion_key(app: &mut App, key: KeyEvent) -> bool {
    if app.completion.is_none() {
        return false;
    }
    match key.code {
        KeyCode::Esc => app.completion = None,
        KeyCode::Up => {
            let menu = app.completion.as_mut().unwrap();
            menu.selected = menu.selected.saturating_sub(1);
        }
        KeyCode::Down => {
            let menu = app.completion.as_mut().unwrap();
            menu.selected = (menu.selected + 1).min(menu.items.len().saturating_sub(1));
        }
        KeyCode::Enter | KeyCode::Tab => {
            let value = app
                .completion
                .as_ref()
                .and_then(|menu| menu.items.get(menu.selected))
                .cloned();
            app.completion = None;
            if let Some(value) = value
                && let Some(editor) = app.tabs.get_mut(app.active)
                && !editor.read_only
            {
                editor.snapshot();
                let before: String = editor.lines[editor.row].chars().take(editor.col).collect();
                let prefix = before
                    .chars()
                    .rev()
                    .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                    .count();
                let start =
                    byte_index(&editor.lines[editor.row], editor.col.saturating_sub(prefix));
                let end = byte_index(&editor.lines[editor.row], editor.col);
                editor.lines[editor.row].replace_range(start..end, &value);
                editor.col = editor.col.saturating_sub(prefix) + value.chars().count();
            }
        }
        _ => {}
    }
    true
}

fn handle_quick_open_key(app: &mut App, key: KeyEvent) -> bool {
    if app.quick_open.is_none() {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => app.quick_open = None,
        KeyCode::Char('p') if ctrl => app.quick_open = None,
        KeyCode::Enter => app.accept_quick_open(),
        KeyCode::Up => {
            let palette = app.quick_open.as_mut().unwrap();
            palette.selected = palette.selected.saturating_sub(1);
        }
        KeyCode::Down => {
            let palette = app.quick_open.as_mut().unwrap();
            palette.selected = (palette.selected + 1).min(palette.results.len().saturating_sub(1));
        }
        KeyCode::Backspace => {
            app.quick_open.as_mut().unwrap().query.pop();
            app.refresh_quick_open();
        }
        KeyCode::Char('u') if ctrl => {
            app.quick_open.as_mut().unwrap().query.clear();
            app.refresh_quick_open();
        }
        KeyCode::Char(ch) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
            app.quick_open.as_mut().unwrap().query.push(ch);
            app.refresh_quick_open();
        }
        _ => {}
    }
    true
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    if handle_prompt_key(app, key)
        || handle_completion_key(app, key)
        || handle_quick_open_key(app, key)
    {
        return;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    if ctrl && shift {
        match key.code {
            KeyCode::Char('a' | 'A') => {
                app.explorer_visible = !app.explorer_visible;
                if !app.explorer_visible && app.focus == Focus::Explorer {
                    app.focus = Focus::Editor;
                }
                app.message = if app.explorer_visible {
                    app.focus = Focus::Explorer;
                    "Explorer shown and focused"
                } else {
                    "Explorer hidden · Ctrl+Shift+A restores it"
                }
                .into();
                return;
            }
            KeyCode::Char('d' | 'D') => {
                app.open_hunk();
                return;
            }
            KeyCode::Char('s' | 'S' | 'f' | 'F' | 'g' | 'G') => {
                app.message =
                    "Editor, Agent, and Terminal visibility require a TIDE workspace".into();
                return;
            }
            _ => {}
        }
    }

    if ctrl {
        match key.code {
            KeyCode::Char('p') => {
                app.start_quick_open();
                return;
            }
            KeyCode::Char(' ') if app.focus == Focus::Editor => {
                app.request_completion();
                return;
            }
            KeyCode::Char('q') => {
                app.request_quit();
                return;
            }
            KeyCode::Char('s') => {
                app.save();
                return;
            }
            KeyCode::Char('r') if app.focus == Focus::Editor => {
                app.reload_active();
                return;
            }
            KeyCode::Char('e') => {
                app.focus = if app.focus == Focus::Explorer && !app.tabs.is_empty() {
                    Focus::Editor
                } else {
                    Focus::Explorer
                };
                return;
            }
            KeyCode::Char('w') if app.focus == Focus::Editor => {
                app.close_tab();
                return;
            }
            KeyCode::Char('h') if app.focus == Focus::Explorer => {
                app.show_hidden = !app.show_hidden;
                app.refresh_tree();
                app.message = if app.show_hidden {
                    "Showing hidden files"
                } else {
                    "Hidden files concealed"
                }
                .into();
                return;
            }
            KeyCode::Char('z') if app.focus == Focus::Editor => {
                if let Some(e) = app.tabs.get_mut(app.active) {
                    e.undo();
                }
                return;
            }
            KeyCode::Char('y') if app.focus == Focus::Editor => {
                if let Some(e) = app.tabs.get_mut(app.active) {
                    e.redo();
                }
                return;
            }
            _ => {}
        }
    }

    if alt && let KeyCode::Char(ch @ '1'..='9') = key.code {
        let index = ch.to_digit(10).unwrap() as usize - 1;
        if index < app.tabs.len() {
            app.active = index;
            app.focus = Focus::Editor;
        }
        return;
    }

    if key.code == KeyCode::F(5) {
        app.refresh_tree();
        app.refresh_git_status();
        app.refresh_diff_summary();
        app.message = "Explorer, Git status, and diff summary refreshed".into();
        return;
    }
    // Unhandled command chords must never fall through to single-letter Explorer
    // actions such as new, rename, move, or delete.
    if ctrl || alt {
        return;
    }
    match app.focus {
        Focus::Explorer => match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_sub(4)
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_add(4)
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => app.activate_selection(),
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => app.collapse_or_parent(),
            KeyCode::Char('[') => app.explorer_scroll_x = app.explorer_scroll_x.saturating_sub(4),
            KeyCode::Char(']') => app.explorer_scroll_x = app.explorer_scroll_x.saturating_add(4),
            KeyCode::Char('n') => app.start_file_action(FileAction::CreateFile),
            KeyCode::Char('N') => app.start_file_action(FileAction::CreateDirectory),
            KeyCode::Char('r') => app.start_file_action(FileAction::Rename),
            KeyCode::Char('m') => app.start_file_action(FileAction::Move),
            KeyCode::Char('d') => app.start_file_action(FileAction::Delete),
            KeyCode::Tab if !app.tabs.is_empty() => app.focus = Focus::Editor,
            _ => {}
        },
        Focus::Editor => {
            let Some(editor) = app.tabs.get_mut(app.active) else {
                app.focus = Focus::Explorer;
                return;
            };
            match key.code {
                KeyCode::Char(ch) if !ctrl && !alt => editor.insert(ch),
                KeyCode::Enter => editor.newline(),
                KeyCode::Backspace => editor.backspace(),
                KeyCode::Delete => editor.delete(),
                KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    editor.scroll_x = editor.scroll_x.saturating_sub(4);
                    editor.col = editor.col.min(editor.line_len());
                }
                KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    editor.scroll_x = editor.scroll_x.saturating_add(4);
                    editor.col = editor.scroll_x.min(editor.line_len());
                }
                KeyCode::Left => {
                    editor.col = editor.col.saturating_sub(1);
                }
                KeyCode::Right => {
                    editor.col = (editor.col + 1).min(editor.line_len());
                }
                KeyCode::Up => {
                    editor.row = editor.row.saturating_sub(1);
                    editor.col = editor.col.min(editor.line_len());
                }
                KeyCode::Down => {
                    editor.row = (editor.row + 1).min(editor.lines.len() - 1);
                    editor.col = editor.col.min(editor.line_len());
                }
                KeyCode::Home => editor.col = 0,
                KeyCode::End => editor.col = editor.line_len(),
                KeyCode::PageUp => {
                    editor.row = editor.row.saturating_sub(20);
                    editor.col = editor.col.min(editor.line_len());
                }
                KeyCode::PageDown => {
                    editor.row = (editor.row + 20).min(editor.lines.len() - 1);
                    editor.col = editor.col.min(editor.line_len());
                }
                KeyCode::Tab => editor.insert_text("    "),
                KeyCode::Esc => app.focus = Focus::Explorer,
                _ => {}
            }
        }
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, width: u16, height: u16) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(Rect::new(0, 0, width, height));
    let pane_constraints = if app.explorer_visible {
        [
            Constraint::Percentage(app.explorer_width),
            Constraint::Percentage(100 - app.explorer_width),
        ]
    } else {
        [Constraint::Length(0), Constraint::Min(1)]
    };
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pane_constraints)
        .split(outer[0]);

    if app.explorer_visible && contains(main[0], mouse.column, mouse.row) {
        match mouse.kind {
            MouseEventKind::ScrollLeft => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_sub(4);
            }
            MouseEventKind::ScrollRight => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_add(4);
            }
            MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_sub(4);
            }
            MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                app.explorer_scroll_x = app.explorer_scroll_x.saturating_add(4);
            }
            MouseEventKind::ScrollUp => app.move_selection(-3),
            MouseEventKind::ScrollDown => app.move_selection(3),
            MouseEventKind::Down(MouseButton::Left) => {
                app.focus = Focus::Explorer;
                let inner = Rect::new(
                    main[0].x + 1,
                    main[0].y + 1,
                    main[0].width.saturating_sub(2),
                    main[0].height.saturating_sub(2),
                );
                if contains(inner, mouse.column, mouse.row) {
                    let rows = app.rows();
                    let selected_index = app
                        .selected
                        .as_ref()
                        .and_then(|p| rows.iter().position(|r| &r.path == p))
                        .unwrap_or(0);
                    let start =
                        selected_index.saturating_sub((inner.height as usize).saturating_sub(1));
                    let index = start + (mouse.row - inner.y) as usize;
                    if let Some(clicked) = rows.get(index) {
                        app.selected = Some(clicked.path.clone());
                        app.activate_selection();
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if !contains(main[1], mouse.column, mouse.row) {
        return;
    }
    let inner = Rect::new(
        main[1].x + 1,
        main[1].y + 1,
        main[1].width.saturating_sub(2),
        main[1].height.saturating_sub(2),
    );
    let diff_width = app.diff_summary.label().width().min(inner.width as usize) as u16;
    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        app.focus = Focus::Editor;
    }
    if mouse.kind == MouseEventKind::Down(MouseButton::Left)
        && mouse.row == inner.y
        && mouse.column >= inner.right().saturating_sub(diff_width)
    {
        app.open_hunk();
        return;
    }
    if app.tabs.is_empty() {
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollLeft => {
            let editor = &mut app.tabs[app.active];
            editor.scroll_x = editor.scroll_x.saturating_sub(4);
            editor.col = editor.col.min(editor.line_len());
        }
        MouseEventKind::ScrollRight => {
            let editor = &mut app.tabs[app.active];
            editor.scroll_x = editor.scroll_x.saturating_add(4);
            editor.col = editor.scroll_x.min(editor.line_len());
        }
        MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
            let editor = &mut app.tabs[app.active];
            editor.scroll_x = editor.scroll_x.saturating_sub(4);
            editor.col = editor.col.min(editor.line_len());
        }
        MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
            let editor = &mut app.tabs[app.active];
            editor.scroll_x = editor.scroll_x.saturating_add(4);
            editor.col = editor.scroll_x.min(editor.line_len());
        }
        MouseEventKind::ScrollUp => {
            let editor = &mut app.tabs[app.active];
            editor.row = editor.row.saturating_sub(3);
            editor.col = editor.col.min(editor.line_len());
        }
        MouseEventKind::ScrollDown => {
            let editor = &mut app.tabs[app.active];
            editor.row = (editor.row + 3).min(editor.lines.len() - 1);
            editor.col = editor.col.min(editor.line_len());
        }
        MouseEventKind::Down(MouseButton::Left) => {
            app.focus = Focus::Editor;
            if mouse.row == inner.y {
                let mut x = inner.x;
                for index in 0..app.tabs.len() {
                    let label = app.tabs[index].tab_label();
                    let end = x.saturating_add(label.width() as u16);
                    if mouse.column >= x && mouse.column < end {
                        let close_start = end.saturating_sub(2);
                        if mouse.column >= close_start {
                            app.close_tab_at(index);
                        } else {
                            app.active = index;
                        }
                        return;
                    }
                    x = end.saturating_add(1);
                }
                return;
            }
            let text_y = inner.y + 1;
            if mouse.row >= text_y && mouse.row < inner.bottom() {
                let editor = &mut app.tabs[app.active];
                let number_width = editor.lines.len().to_string().len().max(3);
                let row = editor.scroll_y + (mouse.row - text_y) as usize;
                if row < editor.lines.len() {
                    editor.row = row;
                    let text_x = inner.x + number_width as u16 + 1;
                    let clicked_col = mouse.column.saturating_sub(text_x) as usize;
                    editor.col = (editor.scroll_x + clicked_col).min(editor.line_len());
                }
            }
        }
        _ => {}
    }
}

fn draw(app: &mut App, frame: &mut Frame) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());
    let pane_constraints = if app.explorer_visible {
        [
            Constraint::Percentage(app.explorer_width),
            Constraint::Percentage(100 - app.explorer_width),
        ]
    } else {
        [Constraint::Length(0), Constraint::Min(1)]
    };
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pane_constraints)
        .split(outer[0]);

    if app.explorer_visible {
        draw_explorer(app, frame, main[0]);
    }
    draw_editor(app, frame, main[1]);

    let focus = if app.focus == Focus::Explorer {
        "EXPLORER"
    } else {
        "EDITOR"
    };
    let status = Line::from(vec![
        Span::styled(
            format!(" {focus} "),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {}", app.message)),
    ]);
    frame.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::Gray)),
        outer[1],
    );
    if app.quick_open.is_some() {
        draw_quick_open(app, frame);
    } else if app.prompt.is_some() {
        draw_file_prompt(app, frame);
    } else if app.completion.is_some() {
        draw_completion(app, frame);
    }
}

fn centered_rect(screen: Rect, width_percent: u16, height: u16) -> Rect {
    let width = (screen.width * width_percent / 100).clamp(30.min(screen.width), screen.width);
    let height = height.min(screen.height);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

fn draw_file_prompt(app: &App, frame: &mut Frame) {
    let Some(prompt) = app.prompt.as_ref() else {
        return;
    };
    let title = match prompt.action {
        FileAction::CreateFile => " CREATE FILE ",
        FileAction::CreateDirectory => " CREATE DIRECTORY ",
        FileAction::Rename => " RENAME ",
        FileAction::Move => " MOVE · destination relative to project ",
        FileAction::Delete => " DELETE · type delete to confirm ",
    };
    let area = centered_rect(frame.area(), 68, 3);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(prompt.input.as_str()), inner);
    frame.set_cursor_position((
        (inner.x + prompt.input.width() as u16).min(inner.right().saturating_sub(1)),
        inner.y,
    ));
}

fn draw_completion(app: &App, frame: &mut Frame) {
    let Some(menu) = app.completion.as_ref() else {
        return;
    };
    let height = (menu.items.len() as u16 + 2).clamp(4, 14);
    let area = centered_rect(frame.area(), 52, height);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" LSP COMPLETION ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let visible = inner.height as usize;
    let start = menu.selected.saturating_sub(visible.saturating_sub(1));
    let lines = menu
        .items
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, item)| {
            let style = if index == menu.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(format!(" {item}"), style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_quick_open(app: &App, frame: &mut Frame) {
    let Some(palette) = app.quick_open.as_ref() else {
        return;
    };
    let screen = frame.area();
    let max_width = screen.width.saturating_sub(2).max(1);
    let minimum_width = 40.min(max_width);
    let width = (screen.width * 72 / 100).max(minimum_width).min(max_width);
    let desired_height = (palette.results.len() as u16 + 4).clamp(8, 20);
    let height = desired_height.min(screen.height.saturating_sub(2));
    let area = Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + 2.min(screen.height.saturating_sub(height) / 3),
        width,
        height,
    );
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" QUICK OPEN · prefix % to search file contents ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);
    let mode = if palette.query.starts_with('%') {
        "CONTENT"
    } else {
        "FILES"
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {mode} "),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::raw(format!(" {}", palette.query)),
        ])),
        parts[0],
    );

    let visible_height = parts[1].height as usize;
    let start = palette
        .selected
        .saturating_sub(visible_height.saturating_sub(1));
    let lines: Vec<Line> = palette
        .results
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_height)
        .map(|(index, result)| {
            let style = if index == palette.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(format!(" {}", result.label), style)
        })
        .collect();
    if lines.is_empty() {
        let hint = if palette.query == "%" {
            " Type after % to search inside project files"
        } else {
            " No matching files"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
            parts[1],
        );
    } else {
        frame.render_widget(Paragraph::new(lines), parts[1]);
    }

    let query_width = palette.query.width() as u16;
    let cursor_x =
        (parts[0].x + mode.len() as u16 + 4 + query_width).min(parts[0].right().saturating_sub(1));
    frame.set_cursor_position((cursor_x, parts[0].y));
}

fn draw_explorer(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Explorer;
    let border = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let title = app
        .root
        .file_name()
        .unwrap_or(app.root.as_os_str())
        .to_string_lossy();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" EXPLORER · {title} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = app.rows();
    let selected_index = app
        .selected
        .as_ref()
        .and_then(|p| rows.iter().position(|r| &r.path == p))
        .unwrap_or(0);
    let height = inner.height as usize;
    let start = selected_index.saturating_sub(height.saturating_sub(1));
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(start)
        .take(height)
        .map(|(index, row)| {
            let name = row.path.file_name().unwrap_or_default().to_string_lossy();
            let icon = if row.is_dir {
                if app.expanded.contains(&row.path) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let git_marker = app.git_status.get(&row.path).copied();
            let git_color = match git_marker {
                Some('?') | Some('A') => Some(Color::Green),
                Some('M') => Some(Color::Yellow),
                Some('D') => Some(Color::Red),
                _ => None,
            };
            let foreground = git_color.unwrap_or(if row.is_dir {
                Color::Blue
            } else {
                Color::White
            });
            let style = if index == selected_index {
                Style::default().fg(foreground).bg(Color::Rgb(20, 65, 72))
            } else {
                Style::default().fg(foreground)
            };
            let marker = git_marker
                .map(|value| format!("{value} "))
                .unwrap_or_else(|| "  ".into());
            let full = format!("{}{}{}{}", "  ".repeat(row.depth), icon, marker, name);
            let visible: String = full.chars().skip(app.explorer_scroll_x).collect();
            Line::styled(visible, style)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
    if rows.len() > height {
        let mut scrollbar = ScrollbarState::new(rows.len())
            .position(selected_index)
            .viewport_content_length(height);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar,
        );
    }
    let widest = rows
        .iter()
        .map(|row| {
            row.depth * 2
                + 4
                + row
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .width()
        })
        .max()
        .unwrap_or(0);
    if widest > inner.width as usize {
        let mut scrollbar = ScrollbarState::new(widest)
            .position(app.explorer_scroll_x)
            .viewport_content_length(inner.width as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom),
            area,
            &mut scrollbar,
        );
    }
}

fn draw_editor(app: &mut App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Editor;
    let has_conflict = app.tabs.get(app.active).is_some_and(|e| e.conflict);
    let border = if has_conflict {
        Color::Red
    } else if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let editor_title = if has_conflict {
        " EDITOR · DISK CONFLICT ".to_string()
    } else if app
        .tabs
        .get(app.active)
        .is_some_and(|editor| editor.visual.is_some())
    {
        format!(" EDITOR · {} MEDIA PREVIEW ", app.image_protocol_name)
    } else {
        " EDITOR ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(editor_title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let editor_parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    let diff_label = app.diff_summary.label();
    let header_parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(diff_label.width().min(inner.width as usize) as u16),
        ])
        .split(editor_parts[0]);
    let diff_style = if app.diff_summary.files == 0 {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(diff_label).style(diff_style),
        header_parts[1],
    );

    if app.tabs.is_empty() {
        let welcome = Text::from(vec![
            Line::from("TIDE"),
            Line::from(""),
            Line::from("Select a file in Explorer and press Enter."),
            Line::from("Click Δ DIFF or press Ctrl+Shift+D to review with Hunk."),
        ]);
        frame.render_widget(
            Paragraph::new(welcome).style(Style::default().fg(Color::DarkGray)),
            editor_parts[1],
        );
        return;
    }

    let tab_titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|editor| Line::from(editor.tab_label()))
        .collect();
    let tabs = Tabs::new(tab_titles)
        .select(app.active)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(Color::DarkGray))
        .divider("│");
    frame.render_widget(tabs, header_parts[0]);

    let text_area = editor_parts[1];
    let syntax_set = &app.syntax_set;
    let syntax_theme = &app.syntax_theme;
    let active_path = app.tabs[app.active].path.clone();
    let diagnostics = app
        .diagnostics
        .get(&active_path)
        .cloned()
        .unwrap_or_default();
    let editor = &mut app.tabs[app.active];
    if let Some(protocol) = editor.visual_protocol.as_mut() {
        frame.render_stateful_widget(
            StatefulImage::new().resize(Resize::Fit(None)),
            text_area,
            protocol,
        );
        return;
    }
    if let Some(image) = &editor.visual {
        draw_visual_preview(frame, text_area, image);
        return;
    }
    let number_width = editor.lines.len().to_string().len().max(3);
    let visible_height = text_area.height as usize;
    let visible_width = text_area.width.saturating_sub(number_width as u16 + 2) as usize;

    if editor.row < editor.scroll_y {
        editor.scroll_y = editor.row;
    }
    if editor.row >= editor.scroll_y + visible_height {
        editor.scroll_y = editor.row + 1 - visible_height;
    }
    if editor.col < editor.scroll_x {
        editor.scroll_x = editor.col;
    }
    if editor.col >= editor.scroll_x + visible_width.max(1) {
        editor.scroll_x = editor.col + 1 - visible_width.max(1);
    }

    let syntax = syntax_set
        .find_syntax_for_file(&editor.path)
        .ok()
        .flatten()
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, syntax_theme);
    let mut lines = Vec::with_capacity(visible_height);
    // Highlight only the visible viewport. Replaying syntax from line zero on
    // every keystroke makes large files unnecessarily expensive.
    for (index, value) in editor
        .lines
        .iter()
        .enumerate()
        .skip(editor.scroll_y)
        .take(visible_height)
    {
        let source_line = format!("{value}\n");
        let highlighted = highlighter.highlight_line(&source_line, syntax_set).ok();
        let changed = editor.externally_changed.contains(&index);
        let diagnostic = diagnostics.iter().find(|item| item.line == index);
        let gutter = if diagnostic.is_some_and(|item| item.severity <= 1) {
            Color::Red
        } else if diagnostic.is_some() {
            Color::Yellow
        } else if changed {
            Color::Green
        } else {
            Color::DarkGray
        };
        let marker = if diagnostic.is_some_and(|item| item.severity <= 1) {
            "!"
        } else if diagnostic.is_some() {
            "?"
        } else if changed {
            "+"
        } else {
            " "
        };
        let mut spans = vec![Span::styled(
            format!("{:>width$}{marker}", index + 1, width = number_width),
            Style::default().fg(gutter),
        )];
        if let Some(ranges) = highlighted {
            let mut character_index = 0;
            for (syntax_style, part) in ranges {
                let mut visible = String::new();
                for ch in part.chars() {
                    if ch == '\n' {
                        continue;
                    }
                    if character_index >= editor.scroll_x {
                        visible.push(ch);
                    }
                    character_index += 1;
                }
                if visible.is_empty() {
                    continue;
                }
                let color = syntax_style.foreground;
                let mut style = Style::default().fg(Color::Rgb(color.r, color.g, color.b));
                if changed {
                    style = style.bg(Color::Rgb(0, 40, 24));
                }
                spans.push(Span::styled(visible, style));
            }
        } else {
            let visible: String = value.chars().skip(editor.scroll_x).collect();
            let style = if changed {
                Style::default()
                    .fg(Color::LightGreen)
                    .bg(Color::Rgb(0, 40, 24))
            } else {
                Style::default()
            };
            spans.push(Span::styled(visible, style));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), text_area);
    if editor.lines.len() > visible_height {
        let mut scrollbar = ScrollbarState::new(editor.lines.len())
            .position(editor.row)
            .viewport_content_length(visible_height);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            text_area,
            &mut scrollbar,
        );
    }
    let longest_line = editor
        .lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if longest_line > visible_width {
        let mut scrollbar = ScrollbarState::new(longest_line)
            .position(editor.scroll_x)
            .viewport_content_length(visible_width);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::HorizontalBottom),
            text_area,
            &mut scrollbar,
        );
    }

    if focused {
        if let Some(diagnostic) = diagnostics.iter().find(|item| item.line == editor.row) {
            let label = format!(
                " LSP {}:{} · {} ",
                diagnostic.line + 1,
                diagnostic.column + 1,
                diagnostic.message
            );
            let y = text_area.bottom().saturating_sub(1);
            frame.render_widget(
                Paragraph::new(label).style(
                    Style::default()
                        .fg(if diagnostic.severity <= 1 {
                            Color::LightRed
                        } else {
                            Color::Yellow
                        })
                        .bg(Color::Black),
                ),
                Rect::new(text_area.x, y, text_area.width, 1),
            );
        }
        let prefix = format!("{:>width$} ", editor.row + 1, width = number_width);
        let before: String = editor.lines[editor.row]
            .chars()
            .skip(editor.scroll_x)
            .take(editor.col.saturating_sub(editor.scroll_x))
            .collect();
        let x = text_area.x + prefix.width() as u16 + before.width() as u16;
        let y = text_area.y + editor.row.saturating_sub(editor.scroll_y) as u16;
        if x < text_area.right() && y < text_area.bottom() {
            frame.set_cursor_position((x, y));
        }
    }
}

fn draw_visual_preview(frame: &mut Frame, area: Rect, image: &DynamicImage) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (source_width, source_height) = image.dimensions();
    let scale = (area.width as f32 / source_width as f32)
        .min((area.height as f32 * 2.0) / source_height as f32);
    let width = ((source_width as f32 * scale).round() as u32).max(1);
    let height = ((source_height as f32 * scale).round() as u32).max(1);
    let resized = image
        .resize_exact(width, height, FilterType::Triangle)
        .to_rgb8();
    let top_padding = (area.height as usize).saturating_sub(height.div_ceil(2) as usize) / 2;
    let left_padding = (area.width as usize).saturating_sub(width as usize) / 2;
    let mut lines = vec![Line::from(""); top_padding];

    for y in (0..height).step_by(2) {
        let mut spans = Vec::with_capacity(width as usize + 1);
        if left_padding > 0 {
            spans.push(Span::raw(" ".repeat(left_padding)));
        }
        for x in 0..width {
            let top = resized.get_pixel(x, y);
            let bottom = resized.get_pixel(x, (y + 1).min(height - 1));
            spans.push(Span::styled(
                "▀",
                Style::default()
                    .fg(Color::Rgb(top[0], top[1], top[2]))
                    .bg(Color::Rgb(bottom[0], bottom[1], bottom[2])),
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn run_hunk_foreground(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root: &Path,
) -> io::Result<std::process::ExitStatus> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    let result = Command::new("hunk")
        .args(["diff", "--watch"])
        .current_dir(root)
        .status();

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.clear()?;
    result
}

fn run() -> io::Result<()> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let root = fs::canonicalize(root)?;
    if !root.is_dir() {
        return Err(io::Error::other("project path is not a directory"));
    }
    let config = TideConfig::load(&root).map_err(io::Error::other)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let requested_protocol = env::var("TIDE_IMAGE_PROTOCOL").ok();
    // A forced protocol is used by Omarchy's tdl setup. Avoid capability queries
    // in that case: multiplexers can consume normal key input while waiting for
    // a terminal response. Yazi follows the same environment-driven approach.
    let mut image_picker = if requested_protocol.is_some() {
        Picker::from_fontsize((8, 16))
    } else {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)))
    };
    match requested_protocol.as_deref() {
        Some("sixel") => image_picker.set_protocol_type(ProtocolType::Sixel),
        Some("kitty") => image_picker.set_protocol_type(ProtocolType::Kitty),
        Some("iterm2") => image_picker.set_protocol_type(ProtocolType::Iterm2),
        Some("halfblocks") => image_picker.set_protocol_type(ProtocolType::Halfblocks),
        _ => {}
    }
    let mut app = App::new(root, image_picker, config);

    let result = (|| -> io::Result<()> {
        let mut needs_redraw = true;
        while !app.quit {
            needs_redraw |= app.watch_open_files();
            if needs_redraw {
                terminal.draw(|frame| draw(&mut app, frame))?;
                needs_redraw = false;
            }
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) => {
                        handle_key(&mut app, key);
                        app.sync_active_lsp();
                        needs_redraw = true;
                    }
                    Event::Mouse(mouse) => {
                        let (width, height) = crossterm::terminal::size()?;
                        handle_mouse(&mut app, mouse, width, height);
                        needs_redraw = true;
                    }
                    Event::Paste(text) if app.prompt.is_some() => {
                        app.prompt
                            .as_mut()
                            .unwrap()
                            .input
                            .push_str(&text.replace(['\r', '\n'], ""));
                        needs_redraw = true;
                    }
                    Event::Paste(text) if app.quick_open.is_some() => {
                        app.quick_open.as_mut().unwrap().query.push_str(&text);
                        app.refresh_quick_open();
                        needs_redraw = true;
                    }
                    Event::Paste(text) if app.focus == Focus::Editor => {
                        if let Some(editor) = app.tabs.get_mut(app.active) {
                            editor.insert_text(&text);
                        }
                        needs_redraw = true;
                    }
                    Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => {
                        needs_redraw = true;
                    }
                    _ => {}
                }
            }
            if std::mem::take(&mut app.launch_hunk_foreground) {
                match run_hunk_foreground(&mut terminal, &app.root) {
                    Ok(status) if status.success() => {
                        app.message = "Closed Hunk review".into();
                    }
                    Ok(status) => app.message = format!("Hunk exited with {status}"),
                    Err(error) => app.message = format!("Could not run Hunk: {error}"),
                }
                app.refresh_git_status();
                app.refresh_diff_summary();
                needs_redraw = true;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn main() {
    if let Err(error) = run() {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        eprintln!("tide-editor: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_paths_cannot_escape_root() {
        let root = std::env::temp_dir().join(format!("tide-path-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        assert_eq!(
            safe_project_path(&root, &root.join("src/../README.md")).unwrap(),
            root.join("README.md")
        );
        assert!(safe_project_path(&root, &root.join("../../etc/passwd")).is_err());
        assert!(safe_project_path(&root, &root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fuzzy_search_prefers_direct_matches() {
        assert!(
            fuzzy_score("src/main.rs", "main").unwrap()
                < fuzzy_score("my/application/index.rs", "main").unwrap()
        );
        assert!(fuzzy_score("README.md", "xyz").is_none());
    }

    #[test]
    fn command_modifiers_do_not_trigger_explorer_file_actions() {
        let root = std::env::temp_dir().join(format!("tide-keys-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut config = TideConfig::default();
        config.lsp.disabled = true;
        let picker = Picker::from_fontsize((8, 16));
        let mut app = App::new(root.clone(), picker, config);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );
        assert!(app.prompt.is_none());

        app.selected = Some(root.clone());
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        );
        app.prompt.as_mut().unwrap().input = "delete".into();
        app.apply_file_action();
        assert!(root.exists());
        assert!(app.message.contains("project root cannot be deleted"));
        fs::remove_dir_all(root).unwrap();
    }
}
