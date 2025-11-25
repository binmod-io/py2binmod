use std::{time::Duration, future::Future, path::Path, fmt::Display, sync::{Arc, Mutex}, io};
use async_trait::async_trait;
use console::{style, StyledObject, Term};
use indicatif::{ProgressBar, ProgressStyle};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet as SyntectThemeSet, Style as SyntectStyle};
use syntect::easy::HighlightLines;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use once_cell::sync::Lazy;

use crate::compiler::traits::OutputSink;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<SyntectThemeSet> = Lazy::new(|| SyntectThemeSet::load_defaults());


pub struct Syntax;

impl Syntax {
    pub fn get_theme(name: &str) -> SyntectTheme {
        THEME_SET
            .themes
            .get(name)
            .cloned()
            .unwrap_or_else(|| THEME_SET.themes["base16-ocean.dark"].clone())
    }

    pub fn get_syntax(path: &Path) -> Option<&'static SyntaxReference> {
        SYNTAX_SET
            .find_syntax_for_file(path)
            .ok()
            .flatten()
    }

    pub fn highlight(code: &str, syntax: &SyntaxReference, theme: &SyntectTheme) -> String {
        let mut h = HighlightLines::new(syntax, theme);
        let mut highlighted = String::new();

        for (i, line) in LinesWithEndings::from(code).enumerate() {
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &SYNTAX_SET).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            highlighted.push_str(&format!("{:>4} | {}", i + 1, escaped));
        }

        highlighted
    }

    pub fn code(code: &str, path: &Path) -> String {
        Self::highlight(
            code,
            Self::get_syntax(path)
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text()),
            &Self::get_theme("InspiredGitHub"),
        )
    }
}


pub struct Style;

impl Style {
    pub fn header(text: &str) -> StyledObject<&str> {
        style(text).bold().cyan()
    }

    pub fn success(text: &str) -> StyledObject<&str> {
        style(text).bold().green()
    }

    pub fn warning(text: &str) -> StyledObject<&str> {
        style(text).yellow()
    }

    pub fn error(text: &str) -> StyledObject<&str> {
        style(text).bold().red()
    }

    pub fn dim(text: &str) -> StyledObject<&str> {
        style(text).dim()
    }

    pub fn key(text: &str) -> StyledObject<&str> {
        style(text).bold()
    }
}

pub struct Printer;

impl Printer {
    pub fn render_section(title: &str) -> String {
        format!("{} {}", Style::header("▶"), Style::header(title))
    }

    pub fn section(title: &str) {
        println!("{}", Self::render_section(title));
    }

    pub fn render_subsection(title: &str) -> String {
        format!("{} {}", Style::dim("›"), Style::header(title))
    }

    pub fn subsection(title: &str) {
        println!("{}", Self::render_subsection(title));
    }

    pub fn render_info(message: &str) -> String {
        format!("{} {}", Style::dim("•"), message)
    }

    pub fn info(message: &str) {
        println!("{}", Self::render_info(message));
    }

    pub fn render_success(message: &str) -> String {
        format!("{} {}", Style::success("✔"), Style::success(message))
    }

    pub fn success(message: &str) {
        println!("{}", Self::render_success(message));
    }

    pub fn render_warning(message: &str) -> String {
        format!("{} {}", Style::warning("!"), Style::warning(message))
    }

    pub fn warning(message: &str) {
        println!("{}", Self::render_warning(message));
    }

    pub fn render_error(message: &str) -> String {
        format!("{} {}", Style::error("✘"), Style::error(message))
    }

    pub fn error(message: &str) {
        println!("{}", Self::render_error(message));
    }
}

pub struct Spinner {
    bar: ProgressBar,
}

impl Spinner {
    pub fn new(message: impl Display) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_message(format!("{}", message));
        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
                .template("{spinner:.dim} {msg}").unwrap(),
        );
        bar.enable_steady_tick(Duration::from_millis(100));

        Spinner { bar }
    }

    pub fn update(&self, message: impl Display) {
        self.bar.set_message(format!("{}", message));
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }

    pub fn finish_with_message(&self, message: impl Display) {
        self.bar.finish_with_message(format!("{}", message));
    }

    pub fn finish_inline(&self, message: impl Display) {
        self.bar.finish_and_clear();
        println!("{}", message);
    }

    pub async fn step<S, FI, T, F, Fut>(start: S, finish: Option<FI>, func: F) -> T
    where 
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
        S: Display,
        FI: Display,
    {
        let spinner = Spinner::new(start);
        let result = func().await;

        if let Some(finish) = finish {
            spinner.finish_inline(finish);
        } else {
            spinner.finish();
        }

        result
    }
}

pub struct Progress {
    bar: ProgressBar,
}

impl Progress {
    pub fn new(total: u64, message: impl Display) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_message(format!("{}", message));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        Progress { bar }
    }

    pub fn increment(&self, value: u64) {
        self.bar.inc(value);
    }

    pub fn finish(&self, message: impl Display) {
        self.bar.finish_with_message(format!("{}", message));
    }
}

pub struct LogPanel {
    title: String,
    height: usize,
    buffer: Mutex<Vec<LogLine>>,
    term: Term,
}

#[derive(Clone)]
pub enum LogKind {
    Stdout,
    Stderr,
}

#[derive(Clone)]
pub struct LogLine {
    pub kind: LogKind,
    pub text: String,
}

impl LogPanel {
    pub fn new(title: impl Display, height: usize) -> Self {
        let term = Term::stdout();
        let this = Self {
            title: format!("{}", title),
            height,
            buffer: Mutex::new(Vec::new()),
            term,
        };

        this.print_title();
        this.reserve_space();
        this
    }

    fn print_title(&self) {
        println!("{}", self.title);
    }

    fn reserve_space(&self) {
        for _ in 0..self.height {
            println!();
        }
    }

    fn push(&self, kind: LogKind, text: impl Into<String>) -> io::Result<()> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(LogLine { kind, text: text.into() });

        if buffer.len() > self.height {
            buffer.remove(0);
        }

        self.redraw(&buffer)
    }

    pub fn append_stdout(&self, text: impl Into<String>) -> io::Result<()> {
        self.push(LogKind::Stdout, text)
    }

    pub fn append_stderr(&self, text: impl Into<String>) -> io::Result<()> {
        self.push(LogKind::Stderr, text)
    }

    fn redraw(&self, buffer: &Vec<LogLine>) -> io::Result<()> {
        self.term.move_cursor_up(self.height)?;

        for line in buffer {
            self.term.clear_line()?;

            match line.kind {
                LogKind::Stdout => {
                    println!("{}", style(&line.text).color256(245));
                }
                LogKind::Stderr => {
                    println!("{}", style(&line.text).color256(245));
                }
            }
        }

        for _ in buffer.len()..self.height {
            self.term.clear_line()?;
            println!();
        }

        Ok(())
    }

    pub fn finish_success(&self, message: impl Display) -> io::Result<()> {
        self.term.clear_last_lines(self.height + 1)?;
        println!("{}", message);

        Ok(())
    }

    pub fn finish_failure(&self, message: impl Display) -> io::Result<()> {
        self.term.move_cursor_up(self.height + 1)?;
        self.term.clear_line()?;
        println!("{}", message);
        self.term.move_cursor_down(self.height)?;

        let buffer = self.buffer.lock().unwrap();
        self.redraw(
            &buffer
                .clone()
                .into_iter()
                .filter(|line| matches!(line.kind, LogKind::Stderr))
                .collect()
        )?;

        Ok(())
    }

    pub async fn step<S, FI, FA, T, F, Fut>(
        title: S,
        height: usize,
        finish: Option<FI>,
        fail: Option<FA>,
        func: F
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
    where 
        F: FnOnce(Arc<LogPanel>) -> Fut,
        Fut: Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
        S: Display,
        FI: Display,
        FA: Display,
    {
        let panel = Arc::new(LogPanel::new(title, height));
        
        match func(panel.clone()).await {
            Ok(result) => {
                panel.finish_success(
                    finish
                        .map(|m| format!("{}", m))
                        .unwrap_or_else(|| "success".to_string())
                )?;

                Ok(result)
            },
            Err(e) => {
                panel.finish_failure(
                    fail
                        .map(|m| format!("{}", m))
                        .unwrap_or_else(|| "error".to_string())
                )?;
            
                Err(e)
            }
        }
    }
}

#[async_trait]
impl OutputSink for LogPanel {
    async fn stdout(&self, line: &str) {
        let _ = self.append_stdout(line);
    }

    async fn stderr(&self, line: &str) {
        let _ = self.append_stderr(line);
    }
}

#[async_trait]
impl OutputSink for Arc<LogPanel> {
    async fn stdout(&self, line: &str) {
        (**self).stdout(line).await
    }

    async fn stderr(&self, line: &str) {
        (**self).stderr(line).await
    }
}