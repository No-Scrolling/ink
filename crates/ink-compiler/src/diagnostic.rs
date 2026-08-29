use std::{fmt, path::Path};

use oxc::span::Span;

#[derive(Debug)]
pub struct CompileError {
    message: String,
    span: Span,
    help: Option<String>,
}

impl CompileError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn render(&self, path: &Path, source: &str) -> String {
        let start = self.span.start as usize;
        let end = self.span.end as usize;
        let line_start = source[..start.min(source.len())]
            .rfind('\n')
            .map_or(0, |position| position + 1);
        let line_end = source[start.min(source.len())..]
            .find('\n')
            .map_or(source.len(), |position| start + position);
        let line_number = source[..line_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let column = source[line_start..start.min(source.len())].chars().count() + 1;
        let line = &source[line_start..line_end];
        let underline_start = source[line_start..start.min(line_end)].chars().count();
        let underline_end = source[line_start..end.min(line_end)].chars().count();
        let underline_length = underline_end.saturating_sub(underline_start).max(1);
        let mut rendered = format!(
            "{}:{}:{}: error: {}\n  |\n{:>3} | {}\n  | {}{}",
            path.display(),
            line_number,
            column,
            self.message,
            line_number,
            line,
            " ".repeat(underline_start),
            "^".repeat(underline_length),
        );

        if let Some(help) = &self.help {
            rendered.push_str("\n  = help: ");
            rendered.push_str(help);
        }

        rendered
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}
