pub mod auth;
pub mod blocks;
pub mod comments;
pub mod common;
pub mod databases;
pub mod markdown;
pub mod pages;
pub mod search;
pub mod users;

use crate::cli::common::GlobalArgs;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "notion-mg",
    about = "A CLI tool for the Notion API, designed for AI agent consumption",
    version
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage authentication
    #[command(subcommand)]
    Auth(auth::AuthAction),

    /// Work with pages
    #[command(subcommand)]
    Pages(pages::PagesAction),

    /// Work with databases
    #[command(subcommand)]
    Databases(databases::DatabasesAction),

    /// Work with blocks
    #[command(subcommand)]
    Blocks(blocks::BlocksAction),

    /// Work with users
    #[command(subcommand)]
    Users(users::UsersAction),

    /// Work with comments
    #[command(subcommand)]
    Comments(comments::CommentsAction),

    /// Convert Markdown into Notion blocks
    #[command(subcommand)]
    Markdown(markdown::MarkdownAction),

    /// Search across pages and databases
    Search(search::SearchArgs),

    /// Raw search with full JSON body
    RawSearch(search::RawSearchArgs),
}
