/// An error returned by the Storykeep API, or a failure to reach it.
///
/// The API answers every error with `{"error": {"code": ..., "message": ...}}`;
/// [status] is the HTTP status (0 when the server could not be reached).
class ApiException implements Exception {
  const ApiException(this.status, this.code, this.message);

  final int status;
  final String code;
  final String message;

  static const networkErrorCode = 'network_error';

  bool get isUnauthorized => status == 401 && code != 'invalid_credentials';
  bool get isForbidden => status == 403;
  bool get isNotFound => status == 404;
  bool get isConflict => status == 409;
  bool get isNetworkError => code == networkErrorCode;

  @override
  String toString() => 'ApiException($status, $code): $message';
}
