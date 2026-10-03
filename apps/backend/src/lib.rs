pub mod auth;
pub mod authz;
pub mod development;
pub mod error;
pub mod measurements;
pub mod profiles;
pub mod reference;
pub mod relationships;
pub mod users;
pub mod validate;

use std::{convert::Infallible, path::PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use sqlx::PgPool;
use warp::{
	Filter,
	filters::BoxedFilter,
	http::StatusCode,
	reply::{Reply, Response},
};

use crate::error::ApiResult;

/// Shared handles passed to every handler.
#[derive(Clone)]
pub struct AppState {
	pub db: PgPool,
	/// Directory where uploaded media files are stored.
	pub media_dir: PathBuf,
}

/// The routes a module contributes, already converted to a uniform response type.
pub type Routes = BoxedFilter<(Response,)>;

/// The full application: `/health` plus the REST API under `/api`.
pub fn app(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
	let api = warp::path("api").and(
		auth::routes(&state)
			.or(users::routes(&state))
			.unify()
			.or(reference_routes(&state))
			.unify()
			.or(profiles::routes(&state))
			.unify()
			.or(relationships::routes(&state))
			.unify()
			.or(development::routes(&state))
			.unify()
			.or(measurements::routes(&state))
			.unify(),
	);

	health(&state)
		.or(api)
		.unify()
		.recover(error::recover)
		.unify()
		.with(warp::trace::request())
}

pub fn with_state(
	state: &AppState,
) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone + use<> {
	let state = state.clone();
	warp::any().map(move || state.clone())
}

/// JSON request body, limited to 64 KiB.
pub fn json_body<T: DeserializeOwned + Send>()
-> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
	warp::body::content_length_limit(64 * 1024).and(warp::body::json())
}

/// Converts a handler's result into a response, so routes can use `.then(handler)`.
pub fn respond<T: Reply>(result: ApiResult<T>) -> Response {
	match result {
		Ok(reply) => reply.into_response(),
		Err(err) => err.into_response(),
	}
}

pub fn json<T: Serialize>(status: StatusCode, body: &T) -> Response {
	warp::reply::with_status(warp::reply::json(body), status).into_response()
}

pub fn ok<T: Serialize>(body: &T) -> ApiResult<Response> {
	Ok(json(StatusCode::OK, body))
}

pub fn created<T: Serialize>(body: &T) -> ApiResult<Response> {
	Ok(json(StatusCode::CREATED, body))
}

pub fn no_content() -> ApiResult<Response> {
	Ok(StatusCode::NO_CONTENT.into_response())
}

fn reference_routes(state: &AppState) -> Routes {
	warp::path!("reference")
		.and(warp::get())
		.and(with_state(state))
		.then(|state: AppState| async move {
			respond(reference::load(&state.db).await.and_then(|data| ok(&data)))
		})
		.boxed()
}

#[derive(Serialize)]
struct Health {
	status: &'static str,
	database: &'static str,
}

fn health(state: &AppState) -> Routes {
	warp::path!("health")
		.and(warp::get())
		.and(with_state(state))
		.then(|state: AppState| async move {
			let (status, body) = match sqlx::query("SELECT 1").execute(&state.db).await {
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
			json(status, &body)
		})
		.boxed()
}
