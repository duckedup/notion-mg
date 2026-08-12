use crate::api::blocks::Block;
use crate::api::comments::{AnchoredComment, Comment};
use crate::api::databases::Database;
use crate::api::pages::Page;
use crate::api::search::SearchResponse;
use crate::api::users::User;

pub trait PrettyPrint {
    fn pretty(&self);
    fn pretty_row(&self) -> Vec<String>;
    fn headers() -> Vec<String>;
}

pub fn print_table(headers: &[String], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{header_line}");
    println!(
        "{}",
        widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  ")
    );

    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                if i < col_count {
                    format!("{:width$}", cell, width = widths[i])
                } else {
                    cell.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!("{line}");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn short_id(id: &str) -> String {
    id.replace('-', "")[..8.min(id.replace('-', "").len())].to_string()
}

impl PrettyPrint for Page {
    fn pretty(&self) {
        println!("Page: {}", self.title());
        println!("  ID:          {}", self.id);
        println!("  Created:     {}", self.created_time);
        println!("  Edited:      {}", self.last_edited_time);
        println!("  Archived:    {}", self.archived);
        if let Some(url) = &self.url {
            println!("  URL:         {url}");
        }
        println!(
            "  Properties:  {}",
            serde_json::to_string_pretty(&self.properties).unwrap_or_default()
        );
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.id),
            truncate(&self.title(), 50),
            self.last_edited_time.clone(),
            if self.archived {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ]
    }

    fn headers() -> Vec<String> {
        vec![
            "ID".to_string(),
            "TITLE".to_string(),
            "EDITED".to_string(),
            "ARCHIVED".to_string(),
        ]
    }
}

impl PrettyPrint for Database {
    fn pretty(&self) {
        println!("Database: {}", self.title_text());
        println!("  ID:          {}", self.id);
        println!("  Created:     {}", self.created_time);
        println!("  Edited:      {}", self.last_edited_time);
        println!("  Inline:      {}", self.is_inline);
        println!("  Archived:    {}", self.archived);
        if let Some(url) = &self.url {
            println!("  URL:         {url}");
        }
        println!(
            "  Properties:  {}",
            serde_json::to_string_pretty(&self.properties).unwrap_or_default()
        );
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.id),
            truncate(&self.title_text(), 50),
            self.last_edited_time.clone(),
            if self.archived {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ]
    }

    fn headers() -> Vec<String> {
        vec![
            "ID".to_string(),
            "TITLE".to_string(),
            "EDITED".to_string(),
            "ARCHIVED".to_string(),
        ]
    }
}

impl PrettyPrint for Block {
    fn pretty(&self) {
        println!("Block: {}", self.block_type);
        println!("  ID:          {}", self.id);
        println!("  Created:     {}", self.created_time);
        println!("  Edited:      {}", self.last_edited_time);
        println!("  Has Children: {}", self.has_children);
        let text = self.plain_text();
        if !text.is_empty() {
            println!("  Text:        {text}");
        }
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.id),
            self.block_type.clone(),
            truncate(&self.plain_text(), 60),
            if self.has_children {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ]
    }

    fn headers() -> Vec<String> {
        vec![
            "ID".to_string(),
            "TYPE".to_string(),
            "TEXT".to_string(),
            "CHILDREN".to_string(),
        ]
    }
}

impl PrettyPrint for User {
    fn pretty(&self) {
        println!("User: {}", self.name.as_deref().unwrap_or("Unknown"));
        println!("  ID:   {}", self.id);
        if let Some(t) = &self.user_type {
            println!("  Type: {t}");
        }
        if let Some(url) = &self.avatar_url {
            println!("  Avatar: {url}");
        }
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.id),
            self.name.clone().unwrap_or_else(|| "Unknown".to_string()),
            self.user_type.clone().unwrap_or_default(),
        ]
    }

    fn headers() -> Vec<String> {
        vec!["ID".to_string(), "NAME".to_string(), "TYPE".to_string()]
    }
}

impl PrettyPrint for Comment {
    fn pretty(&self) {
        println!("Comment: {}", self.id);
        println!("  Created: {}", self.created_time);
        if let Some(disc) = &self.discussion_id {
            println!("  Discussion: {disc}");
        }
        println!("  Text: {}", self.plain_text());
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.id),
            self.created_time.clone(),
            truncate(&self.plain_text(), 60),
        ]
    }

    fn headers() -> Vec<String> {
        vec!["ID".to_string(), "CREATED".to_string(), "TEXT".to_string()]
    }
}

impl PrettyPrint for AnchoredComment {
    fn pretty(&self) {
        self.comment.pretty();
        println!("  Anchor:  {} ({})", self.anchor_id, self.anchor_type);
        if !self.anchor_text.is_empty() {
            println!("  On:      {}", truncate(&self.anchor_text, 80));
        }
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![
            short_id(&self.comment.id),
            self.anchor_type.clone(),
            truncate(&self.anchor_text, 40),
            truncate(&self.comment.plain_text(), 50),
        ]
    }

    fn headers() -> Vec<String> {
        vec![
            "ID".to_string(),
            "ANCHOR".to_string(),
            "ON".to_string(),
            "TEXT".to_string(),
        ]
    }
}

impl PrettyPrint for SearchResponse {
    fn pretty(&self) {
        println!("Search Results: {} items", self.results.len());
        println!("  Has More: {}", self.has_more);
        for (i, result) in self.results.iter().enumerate() {
            let object = result
                .get("object")
                .and_then(|o| o.as_str())
                .unwrap_or("unknown");
            let id = result
                .get("id")
                .and_then(|o| o.as_str())
                .unwrap_or("unknown");
            println!("  [{i}] {object}: {id}");
        }
    }

    fn pretty_row(&self) -> Vec<String> {
        vec![format!("{} results", self.results.len())]
    }

    fn headers() -> Vec<String> {
        vec!["RESULTS".to_string()]
    }
}
