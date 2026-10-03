use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use storykeep_backend::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
		.init();

	let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
	let addr: SocketAddr = env::var("BIND_ADDR")
		.unwrap_or_else(|_| "127.0.0.1:3000".into())
		.parse()
		.context("BIND_ADDR is not a valid socket address")?;
	let media_dir = PathBuf::from(env::var("MEDIA_DIR").unwrap_or_else(|_| "media".into()));

	let db = PgPoolOptions::new()
		.max_connections(10)
		.connect(&database_url)
		.await
		.context("failed to connect to the database")?;

	sqlx::migrate!()
		.run(&db)
		.await
		.context("failed to run database migrations")?;

	tokio::fs::create_dir_all(&media_dir)
		.await
		.with_context(|| format!("failed to create media directory {}", media_dir.display()))?;

	tracing::info!("listening on http://{addr}");
	warp::serve(storykeep_backend::app(AppState { db, media_dir }))
		.bind(addr)
		.await
		.graceful(shutdown_signal())
		.run()
		.await;

	Ok(())
}

async fn shutdown_signal() {
	tokio::signal::ctrl_c()
		.await
		.expect("failed to listen for ctrl-c");
	tracing::info!("shutting down");
}
