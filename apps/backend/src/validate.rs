//! Input validation helpers. Handlers validate before touching the database so
//! clients get specific messages; database constraints remain the backstop.

use chrono::{NaiveDate, Utc};

use crate::error::{ApiError, ApiResult};

/// Trims `value` and checks it is non-empty and at most `max_chars` long.
pub fn required_text(field: &str, value: &str, max_chars: usize) -> ApiResult<String> {
	let value = value.trim();
	if value.is_empty() {
		return Err(ApiError::bad_request(format!("{field} must not be empty")));
	}
	max_length(field, value, max_chars)?;
	Ok(value.to_owned())
}

/// Like [`required_text`] for optional fields; blank strings become `None`.
pub fn optional_text(
	field: &str,
	value: Option<&str>,
	max_chars: usize,
) -> ApiResult<Option<String>> {
	match value.map(str::trim) {
		None | Some("") => Ok(None),
		Some(value) => {
			max_length(field, value, max_chars)?;
			Ok(Some(value.to_owned()))
		}
	}
}

fn max_length(field: &str, value: &str, max_chars: usize) -> ApiResult<()> {
	if value.chars().count() > max_chars {
		return Err(ApiError::bad_request(format!(
			"{field} must be at most {max_chars} characters"
		)));
	}
	Ok(())
}

pub fn not_in_future(field: &str, date: Option<NaiveDate>) -> ApiResult<()> {
	match date {
		Some(date) if date > Utc::now().date_naive() => Err(ApiError::bad_request(format!(
			"{field} must not be in the future"
		))),
		_ => Ok(()),
	}
}

pub fn date_order(
	start_field: &str,
	start: Option<NaiveDate>,
	end_field: &str,
	end: Option<NaiveDate>,
) -> ApiResult<()> {
	match (start, end) {
		(Some(start), Some(end)) if end < start => Err(ApiError::bad_request(format!(
			"{end_field} must not be before {start_field}"
		))),
		_ => Ok(()),
	}
}
