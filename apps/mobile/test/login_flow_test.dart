import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:storykeep/app.dart';
import 'package:storykeep/state/auth_state.dart';

import 'support.dart';

void main() {
  Future<http.Response> backend(http.Request req) async {
    switch (req.url.path) {
      case '/api/auth/login':
        final body = jsonDecode(req.body) as Map<String, dynamic>;
        if (body['password'] != 'correct horse') {
          return errorResponse(401, 'invalid_credentials', 'invalid email or password');
        }
        return jsonResponse(sessionJson());
      case '/api/profiles':
        return jsonResponse([profileJson()]);
      case '/api/relationships':
        return jsonResponse([relationshipJson()]);
    }
    return errorResponse(404, 'not_found', 'not found');
  }

  Future<AuthState> startApp(WidgetTester tester, Future<http.Response> Function(http.Request) handler,
      {String? token}) async {
    final api = fakeApi(handler, token: token);
    final auth = AuthState(api);
    await tester.pumpWidget(StorykeepApp(api: api, auth: auth));
    await auth.bootstrap();
    await tester.pumpAndSettle();
    return auth;
  }

  testWidgets('signing in shows the dashboard grouped by profile', (tester) async {
    final auth = await startApp(tester, backend);
    expect(find.text('Welcome back'), findsOneWidget);

    await tester.enterText(find.byKey(const Key('login-email')), 'ada@example.com');
    await tester.enterText(find.byKey(const Key('login-password')), 'correct horse');
    await tester.tap(find.byKey(const Key('login-submit')));
    await tester.pumpAndSettle();

    expect(auth.status, AuthStatus.authenticated);
    expect(find.text('Hi, Ada'), findsOneWidget);
    expect(find.text('Mira'), findsOneWidget);
    expect(find.text('Parent child'), findsOneWidget);
  });

  testWidgets('wrong credentials show an error and stay on the login screen', (tester) async {
    await startApp(tester, backend);
    await tester.enterText(find.byKey(const Key('login-email')), 'ada@example.com');
    await tester.enterText(find.byKey(const Key('login-password')), 'wrong password');
    await tester.tap(find.byKey(const Key('login-submit')));
    await tester.pumpAndSettle();

    expect(find.text('Wrong email or password.'), findsOneWidget);
    expect(find.text('Welcome back'), findsOneWidget);
  });

  testWidgets('client-side validation blocks an empty form', (tester) async {
    var calls = 0;
    await startApp(tester, (req) async {
      calls++;
      return backend(req);
    });
    await tester.tap(find.byKey(const Key('login-submit')));
    await tester.pumpAndSettle();
    expect(find.text('Email is required'), findsOneWidget);
    expect(find.text('Password is required'), findsOneWidget);
    expect(calls, 0);
  });

  testWidgets('an expired session returns to the login screen', (tester) async {
    final auth = await startApp(tester, (req) async {
      if (req.url.path == '/api/users/me') return jsonResponse(userJson());
      return errorResponse(401, 'unauthorized', 'missing or invalid token');
    }, token: 'stale');
    await tester.pumpAndSettle();

    expect(auth.status, AuthStatus.unauthenticated);
    expect(find.text('Welcome back'), findsOneWidget);
    expect(find.text('Your session has ended. Please sign in again.'), findsOneWidget);
    expect(auth.api.hasToken, isFalse);
  });
}
