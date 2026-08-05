//! Human-readable, optionally colored rendering of a diff.

use std::fmt::Write as _;

use anstyle::{AnsiColor, Color, Style};

use crate::diff::{Change, ChangeKind, Diff, Summary};

const ADDED: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
const REMOVED: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));
const CHANGED: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
const TYPE_CHANGED: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)));
const DIM: Style = Style::new().dimmed();
const BOLD: Style = Style::new().bold();

#[must_use]
pub fn render(diff: &Diff, color: bool) -> String {
    let mut out = String::new();

    if diff.is_empty() {
        paint(&mut out, DIM, "no differences", color);
        out.push('\n');
        return out;
    }

    for change in diff.changes() {
        render_change(&mut out, change, color);
        out.push('\n');
    }

    out.push('\n');
    render_summary(&mut out, &diff.summary(), color);
    out.push('\n');
    out
}

fn render_change(out: &mut String, change: &Change, color: bool) {
    let path = if change.path.is_root() {
        "(root)".to_owned()
    } else {
        change.path.to_string()
    };

    match &change.kind {
        ChangeKind::Added { new } => {
            paint(out, ADDED, "+ ", color);
            paint(out, BOLD, &path, color);
            let _ = write!(out, ": {new}");
        }
        ChangeKind::Removed { old } => {
            paint(out, REMOVED, "- ", color);
            paint(out, BOLD, &path, color);
            let _ = write!(out, ": {old}");
        }
        ChangeKind::Changed { old, new } => {
            paint(out, CHANGED, "~ ", color);
            paint(out, BOLD, &path, color);
            let _ = write!(out, ": {old} ");
            paint(out, DIM, "->", color);
            let _ = write!(out, " {new}");
        }
        ChangeKind::TypeChanged { old, new } => {
            paint(out, TYPE_CHANGED, "! ", color);
            paint(out, BOLD, &path, color);
            out.push_str(": ");
            paint(out, DIM, old.type_name(), color);
            let _ = write!(out, " {old} ");
            paint(out, DIM, "->", color);
            out.push(' ');
            paint(out, DIM, new.type_name(), color);
            let _ = write!(out, " {new}");
        }
    }
}

fn render_summary(out: &mut String, s: &Summary, color: bool) {
    let total = s.added + s.removed + s.changed + s.type_changed;
    let _ = write!(out, "{total} change");
    if total != 1 {
        out.push('s');
    }
    out.push_str(": ");

    let mut parts: Vec<(Style, String)> = Vec::new();
    if s.added > 0 {
        parts.push((ADDED, format!("+{}", s.added)));
    }
    if s.removed > 0 {
        parts.push((REMOVED, format!("-{}", s.removed)));
    }
    if s.changed > 0 {
        parts.push((CHANGED, format!("~{}", s.changed)));
    }
    if s.type_changed > 0 {
        parts.push((TYPE_CHANGED, format!("!{}", s.type_changed)));
    }

    for (i, (style, text)) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        paint(out, *style, text, color);
    }
}

fn paint(out: &mut String, style: Style, text: &str, color: bool) {
    if color {
        let _ = write!(out, "{style}{text}{style:#}");
    } else {
        out.push_str(text);
    }
}
