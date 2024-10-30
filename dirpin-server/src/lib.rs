use axum::{serve, Router};
use eyre::{Context, Result};
use settings::Settings;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;

mod handlers;
mod router;
pub mod settings;

#[cfg(target_family = "unix")]
async fn shutdown_signal() {
    let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("failed to register signal handler");
    let mut interrupt = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("failed to register signal handler");

    tokio::select! {
        _ = term.recv() => {},
        _ = interrupt.recv() => {},
    };
    eprintln!("Shutting down gracefully...");
}

#[cfg(target_family = "windows")]
async fn shutdown_signal() {
    signal::windows::ctrl_c()
        .expect("failed to register signal handler")
        .recv()
        .await;
    eprintln!("Shutting down gracefully...");
}

async fn make_router(_settings: &Settings) -> Router {
    router::router()
}

pub async fn launch(settings: &Settings, address: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(address)
        .await
        .context("Failed to connect to tcp listener")?;
    let r = make_router(&settings).await;

    tracing::info!("Server started at {}", address);
    serve(listener, r.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
