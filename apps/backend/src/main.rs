use std::{convert::Infallible, env, net::SocketAddr};

use anyhow::Context;
use serde::Serialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing_subscriber::EnvFilter;
use warp::{Filter, http::StatusCode, reply::Reply};

#[derive(Serialize)]
struct Health {
	status: &'static str,
	database: &'static str,
}

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

	let pool = PgPoolOptions::new()
		.max_connections(10)
		.connect(&database_url)
		.await
		.context("failed to connect to the database")?;

	sqlx::migrate!()
		.run(&pool)
		.await
		.context("failed to run database migrations")?;

	tracing::info!("listening on http://{addr}");
	warp::serve(routes(pool))
		.bind(addr)
		.await
		.graceful(shutdown_signal())
		.run()
		.await;

	Ok(())
}

fn routes(pool: PgPool) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone {
	let health = warp::path!("health")
		.and(warp::get())
		.and(with_pool(pool))
		.and_then(health);

	health.with(warp::trace::request())
}

fn with_pool(pool: PgPool) -> impl Filter<Extract = (PgPool,), Error = Infallible> + Clone {
	warp::any().map(move || pool.clone())
}

async fn health(pool: PgPool) -> Result<impl Reply, Infallible> {
	let (code, body) = match sqlx::query("SELECT 1").execute(&pool).await {
		Ok(_) => (
			StatusCode::OK,
			Health {
				status: "ok",
				database: "ok",
			},
		),
		Err(err) => {
			tracing::error!("database health check failed: {err}");
			(
				StatusCode::SERVICE_UNAVAILABLE,
				Health {
					status: "degraded",
					database: "unavailable",
				},
			)
		}
	};
	Ok(warp::reply::with_status(warp::reply::json(&body), code))
}

async fn shutdown_signal() {
	tokio::signal::ctrl_c()
		.await
		.expect("failed to listen for ctrl-c");
	tracing::info!("shutting down");
}
