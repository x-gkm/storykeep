use std::convert::Infallible;

use serde::Serialize;
use warp::{
	Rejection,
	http::StatusCode,
	reply::{Reply, Response},
};

pub type ApiResult<T> = Result<T, ApiError>;

/// Errors returned to API clients as `{"error": {"code": ..., "message": ...}}`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
	#[error("{0}")]
	BadRequest(String),
	#[error("authentication required")]
	Unauthorized,
	#[error("invalid email or password")]
	InvalidCredentials,
	#[error("you don't have permission to do this")]
	Forbidden,
	/// Also used for resources the user isn't allowed to see, so their existence isn't leaked.
	#[error("{0} not found")]
	NotFound(&'static str),
	#[error("{0}")]
	Conflict(String),
	#[error("request body is too large")]
	PayloadTooLarge,
	#[error("internal server error")]
	Internal(#[source] anyhow::Error),
}

impl ApiError {
	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::BadRequest(message.into())
	}

	pub fn conflict(message: impl Into<String>) -> Self {
		Self::Conflict(message.into())
	}

	fn status(&self) -> StatusCode {
		match self {
			Self::BadRequest(_) => StatusCode::BAD_REQUEST,
			Self::Unauthorized | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
			Self::Forbidden => StatusCode::FORBIDDEN,
			Self::NotFound(_) => StatusCode::NOT_FOUND,
			Self::Conflict(_) => StatusCode::CONFLICT,
			Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn code(&self) -> &'static str {
		match self {
			Self::BadRequest(_) => "bad_request",
			Self::Unauthorized => "unauthorized",
			Self::InvalidCredentials => "invalid_credentials",
			Self::Forbidden => "forbidden",
			Self::NotFound(_) => "not_found",
			Self::Conflict(_) => "conflict",
			Self::PayloadTooLarge => "payload_too_large",
			Self::Internal(_) => "internal_error",
		}
	}
}

impl warp::reject::Reject for ApiError {}

impl From<sqlx::Error> for ApiError {
	fn from(err: sqlx::Error) -> Self {
		use sqlx::error::ErrorKind;

		match &err {
			sqlx::Error::RowNotFound => Self::NotFound("resource"),
			// Handlers validate input first; these are a backstop for rules only the
			// database can check (uniqueness, references, cross-table triggers).
			sqlx::Error::Database(db) => match db.kind() {
				ErrorKind::UniqueViolation => Self::conflict("a conflicting record already exists"),
				ErrorKind::ForeignKeyViolation => {
					Self::bad_request("a referenced record does not exist")
				}
				ErrorKind::CheckViolation | ErrorKind::NotNullViolation => {
					Self::bad_request(db.message().to_owned())
				}
				_ => Self::Internal(err.into()),
			},
			_ => Self::Internal(err.into()),
		}
	}
}

impl From<anyhow::Error> for ApiError {
	fn from(err: anyhow::Error) -> Self {
		Self::Internal(err)
	}
}

#[derive(Serialize)]
struct ErrorBody<'a> {
	error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
	code: &'a str,
	message: String,
}

fn error_response(status: StatusCode, code: &str, message: String) -> Response {
	let body = ErrorBody {
		error: ErrorDetail { code, message },
	};
	warp::reply::with_status(warp::reply::json(&body), status).into_response()
}

impl Reply for ApiError {
	fn into_response(self) -> Response {
		if let Self::Internal(err) = &self {
			tracing::error!("{err:#}");
		}
		error_response(self.status(), self.code(), self.to_string())
	}
}

/// Turns warp's built-in rejections and our own into JSON error responses.
pub async fn recover(rejection: Rejection) -> Result<Response, Infallible> {
	if let Some(err) = rejection.find::<ApiError>() {
		// Rejections only hand out references; rebuild the response from the parts.
		let message = match err {
			ApiError::Internal(err) => {
				tracing::error!("{err:#}");
				"internal server error".to_owned()
			}
			other => other.to_string(),
		};
		return Ok(error_response(err.status(), err.code(), message));
	}

	let (status, code, message) = if rejection.is_not_found() {
		(
			StatusCode::NOT_FOUND,
			"not_found",
			"route not found".to_owned(),
		)
	} else if let Some(err) = rejection.find::<warp::filters::body::BodyDeserializeError>() {
		(
			StatusCode::BAD_REQUEST,
			"bad_request",
			format!("invalid request body: {err}"),
		)
	} else if rejection.find::<warp::reject::PayloadTooLarge>().is_some() {
		(
			StatusCode::PAYLOAD_TOO_LARGE,
			"payload_too_large",
			"request body is too large".to_owned(),
		)
	} else if rejection
		.find::<warp::reject::UnsupportedMediaType>()
		.is_some()
	{
		(
			StatusCode::UNSUPPORTED_MEDIA_TYPE,
			"unsupported_media_type",
			"unsupported content type".to_owned(),
		)
	} else if let Some(err) = rejection.find::<warp::reject::InvalidQuery>() {
		(
			StatusCode::BAD_REQUEST,
			"bad_request",
			format!("invalid query string: {err}"),
		)
	} else if rejection.find::<warp::reject::MethodNotAllowed>().is_some() {
		(
			StatusCode::METHOD_NOT_ALLOWED,
			"method_not_allowed",
			"method not allowed".to_owned(),
		)
	} else if rejection.find::<warp::reject::MissingHeader>().is_some()
		|| rejection.find::<warp::reject::InvalidHeader>().is_some()
	{
		(
			StatusCode::BAD_REQUEST,
			"bad_request",
			"missing or invalid header".to_owned(),
		)
	} else {
		tracing::error!("unhandled rejection: {rejection:?}");
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			"internal_error",
			"internal server error".to_owned(),
		)
	};
	Ok(error_response(status, code, message))
}
