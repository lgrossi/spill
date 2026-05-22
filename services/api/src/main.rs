use std::net::SocketAddr;

use anyhow::Context;
use axum::{Json, Router, routing::get};
use clap::{Parser, Subcommand};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

#[derive(Parser, Debug)]
#[command(name = "spillio-api")]
#[command(about = "SpillItOut API service")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve {
        #[arg(long, env = "SPILLIO_API_ADDR", default_value = "127.0.0.1:4000")]
        addr: SocketAddr,
    },
    Migrate {
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
    },
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<HealthResponse> {
    Json(health_response())
}

fn health_response() -> HealthResponse {
    HealthResponse {
        status: "ok",
        service: "spillio-api",
    }
}

async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind API listener on {addr}"))?;

    tracing::info!(%addr, "spillio API listening");

    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("API server failed")
}

async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    tracing::info!("database migrations applied");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spillio_api=info,tower_http=info".into()),
        )
        .init();

    match Cli::parse().command {
        Command::Serve { addr } => run_server(addr).await,
        Command::Migrate { database_url } => run_migrations(&database_url).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_identifies_the_api() {
        let response = health_response();

        assert_eq!(response.status, "ok");
        assert_eq!(response.service, "spillio-api");
    }
}
