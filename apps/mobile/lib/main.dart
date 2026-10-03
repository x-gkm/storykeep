import 'package:flutter/material.dart';

import 'api/api_client.dart';
import 'api/secure_token_store.dart';
import 'app.dart';
import 'config.dart';
import 'state/auth_state.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final api = ApiClient(baseUrl: apiBaseUrl(), tokenStore: SecureTokenStore());
  final auth = AuthState(api)..bootstrap();
  runApp(StorykeepApp(api: api, auth: auth));
}
