//! Markdown to Notion block conversion.
//!
//! Covers the CommonMark/GFM subset Notion has block types for. Anything without an
//! equivalent (footnotes, definition lists, raw HTML) falls through to a paragraph.

use serde_json::{Value, json};

/// Notion rejects a single rich text object longer than this, so long runs are split
/// across several objects rather than truncated.
const MAX_RICH_TEXT_LEN: usize = 2000;

/// Notion caps `PATCH /blocks/{id}/children` at 100 blocks per call.
pub const MAX_BLOCKS_PER_REQUEST: usize = 100;

pub fn markdown_to_blocks(markdown: &str) -> Vec<Value> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }

        if let Some((block, next)) = parse_fenced_code(&lines, i)
            .or_else(|| parse_table(&lines, i))
            .or_else(|| parse_heading(&lines, i))
            .or_else(|| parse_divider(&lines, i))
            .or_else(|| parse_image(&lines, i))
            .or_else(|| parse_quote(&lines, i))
        {
            blocks.push(block);
            i = next;
            continue;
        }

        if let Some((list_blocks, next)) = parse_list(&lines, i) {
            blocks.extend(list_blocks);
            i = next;
            continue;
        }

        let (block, next) = parse_paragraph(&lines, i);
        blocks.push(block);
        i = next;
    }

    blocks
}

/// Splits blocks into runs Notion will accept in one append request.
pub fn chunk_blocks(blocks: Vec<Value>) -> Vec<Vec<Value>> {
    blocks
        .chunks(MAX_BLOCKS_PER_REQUEST)
        .map(<[Value]>::to_vec)
        .collect()
}

// Every block-level parser returns its block plus the index to resume from.

fn parse_fenced_code(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    let trimmed = lines[start].trim_start();
    let fence = ["```", "~~~"]
        .into_iter()
        .find(|f| trimmed.starts_with(f))?;

    let language = notion_language(trimmed[fence.len()..].trim());
    let indent = indent_width(lines[start]);

    let mut body = Vec::new();
    let mut i = start + 1;
    while i < lines.len() && !lines[i].trim_start().starts_with(fence) {
        body.push(strip_indent(lines[i], indent));
        i += 1;
    }

    let block = json!({
        "object": "block",
        "type": "code",
        "code": {
            "rich_text": plain_rich_text(&body.join("\n")),
            "language": language,
        }
    });

    // Skip the closing fence, unless the block ran to end of input unterminated.
    Some((block, (i + 1).min(lines.len())))
}

fn parse_heading(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    let trimmed = lines[start].trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = trimmed.get(hashes..)?;
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }

    // Notion stops at heading_3; deeper markdown headings collapse into it.
    let block_type = format!("heading_{}", hashes.min(3));
    let block = json!({
        "object": "block",
        "type": block_type,
        block_type: { "rich_text": parse_inline(rest.trim()) }
    });
    Some((block, start + 1))
}

fn parse_divider(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    let trimmed = lines[start].trim();
    let is_divider = ['-', '*', '_'].into_iter().any(|c| {
        trimmed.len() >= 3 && trimmed.chars().all(|ch| ch == c || ch == ' ') && trimmed.contains(c)
    });
    if !is_divider {
        return None;
    }
    Some((
        json!({ "object": "block", "type": "divider", "divider": {} }),
        start + 1,
    ))
}

fn parse_image(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    let trimmed = lines[start].trim();
    let rest = trimmed.strip_prefix("!")?;
    let chars: Vec<char> = rest.chars().collect();
    let (alt, url, consumed) = parse_link_syntax(&chars, 0)?;
    if consumed != chars.len() {
        return None;
    }

    let mut image = json!({ "type": "external", "external": { "url": url } });
    let alt_text: String = alt.iter().collect();
    if !alt_text.is_empty() {
        image["caption"] = json!(plain_rich_text(&alt_text));
    }

    Some((
        json!({ "object": "block", "type": "image", "image": image }),
        start + 1,
    ))
}

fn parse_quote(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    if !lines[start].trim_start().starts_with('>') {
        return None;
    }

    let mut body = Vec::new();
    let mut i = start;
    while i < lines.len() && lines[i].trim_start().starts_with('>') {
        let text = lines[i].trim_start().trim_start_matches('>');
        body.push(text.strip_prefix(' ').unwrap_or(text));
        i += 1;
    }

    let block = json!({
        "object": "block",
        "type": "quote",
        "quote": { "rich_text": parse_inline(&body.join("\n")) }
    });
    Some((block, i))
}

fn parse_table(lines: &[&str], start: usize) -> Option<(Value, usize)> {
    let header = split_table_row(lines[start])?;
    if !lines.get(start + 1).is_some_and(|l| is_table_delimiter(l)) {
        return None;
    }

    let width = header.len();
    let mut rows = vec![header];
    let mut i = start + 2;
    while i < lines.len() {
        let Some(mut cells) = split_table_row(lines[i]) else {
            break;
        };
        cells.resize(width, String::new());
        rows.push(cells);
        i += 1;
    }

    let children: Vec<Value> = rows
        .iter()
        .map(|cells| {
            let cell_texts: Vec<Value> = cells
                .iter()
                .take(width)
                .map(|c| json!(parse_inline(c)))
                .collect();
            json!({
                "object": "block",
                "type": "table_row",
                "table_row": { "cells": cell_texts }
            })
        })
        .collect();

    let block = json!({
        "object": "block",
        "type": "table",
        "table": {
            "table_width": width,
            "has_column_header": true,
            "has_row_header": false,
            "children": children,
        }
    });
    Some((block, i))
}

fn parse_paragraph(lines: &[&str], start: usize) -> (Value, usize) {
    let mut body = vec![lines[start].trim()];
    let mut i = start + 1;
    while i < lines.len() && !lines[i].trim().is_empty() && !starts_block(lines, i) {
        body.push(lines[i].trim());
        i += 1;
    }

    let block = json!({
        "object": "block",
        "type": "paragraph",
        "paragraph": { "rich_text": parse_inline(&body.join("\n")) }
    });
    (block, i)
}

/// True when the line at `at` opens a block that must not be swallowed by a
/// preceding paragraph's lazy continuation.
fn starts_block(lines: &[&str], at: usize) -> bool {
    let trimmed = lines[at].trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with("~~~")
        || trimmed.starts_with('>')
        || parse_heading(lines, at).is_some()
        || parse_divider(lines, at).is_some()
        || parse_table(lines, at).is_some()
        || list_marker(lines[at]).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Marker {
    Bulleted,
    Numbered,
    Todo(bool),
}

impl Marker {
    fn block_type(self) -> &'static str {
        match self {
            Self::Bulleted => "bulleted_list_item",
            Self::Numbered => "numbered_list_item",
            Self::Todo(_) => "to_do",
        }
    }
}

struct ListItem {
    indent: usize,
    marker: Marker,
    text: String,
}

fn list_marker(line: &str) -> Option<(Marker, &str)> {
    let trimmed = line.trim_start();

    let after_bullet = ["- ", "* ", "+ "]
        .into_iter()
        .find_map(|m| trimmed.strip_prefix(m));

    if let Some(rest) = after_bullet {
        // GFM task list: the checkbox marker sits inside a bullet item.
        for (prefix, checked) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
            if let Some(task) = rest.strip_prefix(prefix) {
                return Some((Marker::Todo(checked), task));
            }
        }
        return Some((Marker::Bulleted, rest));
    }

    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &trimmed[digits..];
        for sep in [". ", ") "] {
            if let Some(text) = rest.strip_prefix(sep) {
                return Some((Marker::Numbered, text));
            }
        }
    }

    None
}

fn parse_list(lines: &[&str], start: usize) -> Option<(Vec<Value>, usize)> {
    list_marker(lines[start])?;

    let mut items = Vec::new();
    let mut i = start;
    while i < lines.len() {
        if let Some((marker, text)) = list_marker(lines[i]) {
            items.push(ListItem {
                indent: indent_width(lines[i]),
                marker,
                text: text.trim_end().to_string(),
            });
            i += 1;
            continue;
        }

        // A blank line only ends the list if no further item follows it.
        if lines[i].trim().is_empty()
            && lines
                .get(i + 1)
                .is_some_and(|next| list_marker(next).is_some())
        {
            i += 1;
            continue;
        }
        break;
    }

    Some((build_list(&items), i))
}

/// Rebuilds indentation as nesting. Items deeper than the run's base indent become
/// children of the item above them.
fn build_list(items: &[ListItem]) -> Vec<Value> {
    let Some(base) = items.iter().map(|it| it.indent).min() else {
        return Vec::new();
    };

    let mut blocks: Vec<Value> = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        let child_start = i + 1;
        let mut child_end = child_start;
        while child_end < items.len() && items[child_end].indent > base {
            child_end += 1;
        }

        let block_type = item.marker.block_type();
        let mut content = json!({ "rich_text": parse_inline(&item.text) });
        if let Marker::Todo(checked) = item.marker {
            content["checked"] = json!(checked);
        }
        let children = build_list(&items[child_start..child_end]);
        if !children.is_empty() {
            content["children"] = json!(children);
        }

        blocks.push(json!({
            "object": "block",
            "type": block_type,
            block_type: content
        }));
        i = child_end;
    }

    blocks
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Annotations {
    bold: bool,
    italic: bool,
    strikethrough: bool,
    code: bool,
}

impl Annotations {
    fn to_json(self) -> Option<Value> {
        (self != Self::default()).then(|| {
            json!({
                "bold": self.bold,
                "italic": self.italic,
                "strikethrough": self.strikethrough,
                "code": self.code,
            })
        })
    }
}

pub fn parse_inline(text: &str) -> Vec<Value> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    parse_spans(&chars, Annotations::default(), None, &mut out);
    out
}

fn parse_spans(chars: &[char], ann: Annotations, link: Option<&str>, out: &mut Vec<Value>) {
    let mut buf = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\\' && i + 1 < chars.len() {
            buf.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if c == '`'
            && let Some(end) = chars[i + 1..].iter().position(|ch| *ch == '`')
        {
            let end = i + 1 + end;
            flush(&mut buf, ann, link, out);
            let inner: String = chars[i + 1..end].iter().collect();
            push_text(&inner, Annotations { code: true, ..ann }, link, out);
            i = end + 1;
            continue;
        }

        if let Some((delim_len, next_ann)) = emphasis_at(chars, i, ann)
            && let Some(end) = find_closer(chars, i + delim_len, &chars[i..i + delim_len])
        {
            flush(&mut buf, ann, link, out);
            parse_spans(&chars[i + delim_len..end], next_ann, link, out);
            i = end + delim_len;
            continue;
        }

        if c == '['
            && let Some((label, url, consumed)) = parse_link_syntax(chars, i)
        {
            flush(&mut buf, ann, link, out);
            parse_spans(&label, ann, Some(&url), out);
            i += consumed;
            continue;
        }

        buf.push(c);
        i += 1;
    }

    flush(&mut buf, ann, link, out);
}

/// Returns the delimiter length and resulting annotations if an emphasis run opens at `i`.
fn emphasis_at(chars: &[char], i: usize, ann: Annotations) -> Option<(usize, Annotations)> {
    let c = chars[i];
    let doubled = chars.get(i + 1) == Some(&c);

    match c {
        '~' if doubled => Some((
            2,
            Annotations {
                strikethrough: true,
                ..ann
            },
        )),
        '*' if doubled => Some((2, Annotations { bold: true, ..ann })),
        '*' => Some((
            1,
            Annotations {
                italic: true,
                ..ann
            },
        )),
        // Underscores only delimit at word boundaries, so snake_case survives intact.
        '_' if i == 0 || !chars[i - 1].is_alphanumeric() => {
            let len = if doubled { 2 } else { 1 };
            Some((
                len,
                if doubled {
                    Annotations { bold: true, ..ann }
                } else {
                    Annotations {
                        italic: true,
                        ..ann
                    }
                },
            ))
        }
        _ => None,
    }
}

fn find_closer(chars: &[char], from: usize, delim: &[char]) -> Option<usize> {
    let mut i = from;
    while i + delim.len() <= chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == '`'
            && let Some(end) = chars[i + 1..].iter().position(|c| *c == '`')
        {
            i += end + 2;
            continue;
        }
        if chars[i..i + delim.len()] == *delim {
            // A run longer than the delimiter (`***bold italic***`) closes the outer
            // emphasis at its tail, leaving the leading characters to close inner spans.
            let run = chars[i..].iter().take_while(|c| **c == delim[0]).count();
            let closer = i + run - delim.len();
            let closes_word =
                delim[0] != '_' || chars.get(i + run).is_none_or(|c| !c.is_alphanumeric());

            if closer > from && closes_word {
                return Some(closer);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

/// Parses `[label](url)` at `at`, returning the label characters, the URL, and how
/// many characters were consumed.
fn parse_link_syntax(chars: &[char], at: usize) -> Option<(Vec<char>, String, usize)> {
    if chars.get(at) != Some(&'[') {
        return None;
    }

    let label_end = matching(chars, at, '[', ']')?;
    if chars.get(label_end + 1) != Some(&'(') {
        return None;
    }
    let url_end = matching(chars, label_end + 1, '(', ')')?;

    let target: String = chars[label_end + 2..url_end].iter().collect();
    // Drop an optional link title: [text](url "title")
    let url = target
        .split_once(char::is_whitespace)
        .map_or(target.as_str(), |(u, _)| u)
        .to_string();
    if url.is_empty() {
        return None;
    }

    Some((chars[at + 1..label_end].to_vec(), url, url_end + 1 - at))
}

fn matching(chars: &[char], open_at: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open_at;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn flush(buf: &mut String, ann: Annotations, link: Option<&str>, out: &mut Vec<Value>) {
    if !buf.is_empty() {
        push_text(buf, ann, link, out);
        buf.clear();
    }
}

fn push_text(text: &str, ann: Annotations, link: Option<&str>, out: &mut Vec<Value>) {
    let chars: Vec<char> = text.chars().collect();
    for chunk in chars.chunks(MAX_RICH_TEXT_LEN) {
        let content: String = chunk.iter().collect();
        let mut rich = json!({
            "type": "text",
            "text": { "content": content, "link": link.map(|url| json!({ "url": url })) }
        });
        if let Some(annotations) = ann.to_json() {
            rich["annotations"] = annotations;
        }
        out.push(rich);
    }
}

fn plain_rich_text(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    push_text(text, Annotations::default(), None, &mut out);
    out
}

fn indent_width(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn strip_indent(line: &str, width: usize) -> &str {
    let mut consumed = 0;
    let mut offset = 0;
    for c in line.chars() {
        if consumed >= width || !c.is_whitespace() {
            break;
        }
        consumed += if c == '\t' { 4 } else { 1 };
        offset += c.len_utf8();
    }
    &line[offset..]
}

fn is_table_delimiter(line: &str) -> bool {
    let Some(cells) = split_table_row(line) else {
        return false;
    };
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim();
            c.len() >= 3 && c.starts_with([':', '-']) && c.chars().all(|ch| ch == '-' || ch == ':')
        })
}

fn split_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 2 {
        return None;
    }
    Some(
        trimmed[1..trimmed.len() - 1]
            .split('|')
            .map(|c| c.trim().to_string())
            .collect(),
    )
}

/// Notion accepts a fixed language list; unknown or absent tags become plain text.
fn notion_language(tag: &str) -> &'static str {
    const LANGUAGES: &[&str] = &[
        "abap",
        "arduino",
        "bash",
        "basic",
        "c",
        "c#",
        "c++",
        "clojure",
        "coffeescript",
        "css",
        "dart",
        "diff",
        "docker",
        "elixir",
        "elm",
        "erlang",
        "f#",
        "flow",
        "fortran",
        "gherkin",
        "glsl",
        "go",
        "graphql",
        "groovy",
        "haskell",
        "html",
        "java",
        "javascript",
        "json",
        "julia",
        "kotlin",
        "latex",
        "less",
        "lisp",
        "livescript",
        "lua",
        "makefile",
        "markdown",
        "markup",
        "matlab",
        "mermaid",
        "nix",
        "objective-c",
        "ocaml",
        "pascal",
        "perl",
        "php",
        "plain text",
        "powershell",
        "prolog",
        "protobuf",
        "python",
        "r",
        "reason",
        "ruby",
        "rust",
        "sass",
        "scala",
        "scheme",
        "scss",
        "shell",
        "sql",
        "swift",
        "typescript",
        "vb.net",
        "verilog",
        "vhdl",
        "visual basic",
        "webassembly",
        "xml",
        "yaml",
    ];

    let tag = tag.trim().to_lowercase();
    let canonical = match tag.as_str() {
        "js" | "node" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "sh" | "zsh" | "console" => "shell",
        "yml" => "yaml",
        "md" => "markdown",
        "dockerfile" => "docker",
        "text" | "txt" | "" => "plain text",
        other => other,
    };

    LANGUAGES
        .iter()
        .find(|l| **l == canonical)
        .copied()
        .unwrap_or("plain text")
}
