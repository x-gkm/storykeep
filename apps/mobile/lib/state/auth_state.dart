import 'package:flutter/foundation.dart';

import '../api/api_client.dart';

enum AuthStatus { unknown, authenticated, unauthenticated }

/// The signed-in user and session lifecycle. The router listens to it to switch
/// between the login screens and the app.
class AuthState extends ChangeNotifier {
  AuthState(this.api) {
    api.onUnauthorized = _onUnauthorized;
  }

  final ApiClient api;

  AuthStatus status = AuthStatus.unknown;
  User? user;

  /// Set when the stored session couldn't be checked (e.g. server unreachable).
  String? startupError;

  /// Message shown on the login screen after the session expired.
  String? sessionMessage;

  /// Restores a stored token and loads the current user.
  Future<void> bootstrap() async {
    startupError = null;
    status = AuthStatus.unknown;
    notifyListeners();
    if (!await api.restoreToken()) {
      _set(AuthStatus.unauthenticated);
      return;
    }
    try {
      user = await api.me();
      _set(AuthStatus.authenticated);
    } on ApiException catch (e) {
      if (e.isUnauthorized) {
        _set(AuthStatus.unauthenticated);
      } else {
        startupError = e.message;
        notifyListeners();
      }
    }
  }

  Future<void> login(String email, String password) async {
    final session = await api.login(email.trim(), password);
    user = session.user;
    sessionMessage = null;
    _set(AuthStatus.authenticated);
  }

  Future<void> register({
    required String email,
    required String password,
    required String firstName,
    required String lastName,
    DateTime? dateOfBirth,
  }) async {
    final session = await api.register(
      email: email.trim(),
      password: password,
      firstName: firstName.trim(),
      lastName: lastName.trim(),
      dateOfBirth: dateOfBirth,
    );
    user = session.user;
    sessionMessage = null;
    _set(AuthStatus.authenticated);
  }

  Future<void> logout() async {
    await api.logout();
    user = null;
    _set(AuthStatus.unauthenticated);
  }

  void updateUser(User u) {
    user = u;
    notifyListeners();
  }

  void _onUnauthorized() {
    user = null;
    sessionMessage = 'Your session has ended. Please sign in again.';
    _set(AuthStatus.unauthenticated);
  }

  void _set(AuthStatus s) {
    status = s;
    notifyListeners();
  }
}
