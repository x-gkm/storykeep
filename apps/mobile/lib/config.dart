import 'dart:io';

/// Server root of the Storykeep API, set with `--dart-define=API_BASE_URL=...`.
///
/// Defaults to the host machine as seen from the Android emulator
/// (`http://10.0.2.2:3000`) or from the iOS simulator (`http://127.0.0.1:3000`).
Uri apiBaseUrl() {
  const fromEnv = String.fromEnvironment('API_BASE_URL');
  if (fromEnv.isNotEmpty) return Uri.parse(fromEnv);
  return Uri.parse(Platform.isAndroid ? 'http://10.0.2.2:3000' : 'http://127.0.0.1:3000');
}
