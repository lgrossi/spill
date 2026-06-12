mod api;
mod auth;
mod gif;
mod model;
mod publish;
mod read;
mod state;
mod update;

use clap::{Parser, Subcommand};

/// Spill retro board companion.
#[derive(Parser)]
#[command(name = "spill", version, about = "Spill retro board companion")]
struct Cli {
    /// API base URL (or SPILLIO_API_URL).
    #[arg(long, global = true)]
    api_url: Option<String>,
    /// Web base URL used for browser login (or SPILLIO_WEB_URL).
    #[arg(long, global = true)]
    web_url: Option<String>,
    /// Bearer token (or SPILLIO_API_TOKEN); bypasses the cached/login token.
    #[arg(long, global = true)]
    token: Option<String>,
    /// Local dev identity header (or SPILLIO_ON_BEHALF_OF); ignored when a token is provided.
    #[arg(long, global = true)]
    on_behalf_of: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the active board, its series window, and columns.
    State,
    /// Push reviewed cards to a board. Requires --confirm (the human gate).
    Publish {
        #[arg(long)]
        retro_id: String,
        /// Cards JSON file (defaults to stdin).
        #[arg(long)]
        file: Option<String>,
        /// Ingestion source label (or SPILLIO_SOURCE), e.g. claude_code or pi.
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        confirm: bool,
    },
    /// Search GIFs for a phrase and print matches as JSON.
    Gif {
        /// Search phrase, e.g. "mic drop".
        query: String,
        /// What to match: gif | sticker | clip | all.
        #[arg(long, default_value = "gif")]
        kind: String,
        /// Maximum number of results to print.
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Print a board's columns and the cards visible to you (JSON).
    Read {
        #[arg(long)]
        retro_id: String,
    },
    /// Print the resolved API bearer token (for scripting).
    Token,
    /// Authenticate via the browser and cache a token.
    Login {
        /// Paste a token from the web app instead of opening a browser.
        #[arg(long)]
        manual: bool,
    },
    /// Clear the cached token.
    Logout,
    /// Update the spill binary to the latest release.
    Update,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("spill: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let api_url = resolve(
        cli.api_url.clone(),
        "SPILLIO_API_URL",
        "http://127.0.0.1:4000",
    );
    let web_url = resolve(
        cli.web_url.clone(),
        "SPILLIO_WEB_URL",
        "http://127.0.0.1:3000",
    );

    // Best-effort, throttled self-update. Never blocks the command on failure;
    // a downloaded update applies on the next run, not the current one.
    if !matches!(cli.command, Command::Update) {
        update::maybe_auto_update();
    }

    match cli.command {
        Command::Login { manual } => auth::login(&web_url, manual),
        Command::Logout => auth::logout(),
        Command::Update => update::update_now(),
        Command::State => {
            let client = api::ApiClient::new(api_url, web_url, cli.token, cli.on_behalf_of)?;
            state::run(&client)
        }
        Command::Publish {
            retro_id,
            file,
            source,
            confirm,
        } => {
            let client = api::ApiClient::new(api_url, web_url, cli.token, cli.on_behalf_of)?;
            publish::run(&client, &retro_id, file.as_deref(), source, confirm)
        }
        Command::Gif { query, kind, limit } => {
            let client = api::ApiClient::new(api_url, web_url, cli.token, cli.on_behalf_of)?;
            gif::run(&client, &query, &kind, limit)
        }
        Command::Read { retro_id } => {
            let client = api::ApiClient::new(api_url, web_url, cli.token, cli.on_behalf_of)?;
            read::run(&client, &retro_id)
        }
        Command::Token => {
            let token = cli
                .token
                .or_else(|| std::env::var("SPILLIO_API_TOKEN").ok())
                .filter(|t| !t.trim().is_empty())
                .map(Ok)
                .unwrap_or_else(|| auth::ensure_token(&web_url))?;
            println!("{token}");
            Ok(())
        }
    }
}

fn resolve(flag: Option<String>, env_key: &str, default: &str) -> String {
    flag.or_else(|| std::env::var(env_key).ok())
        .map(|v| v.trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}
