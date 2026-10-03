import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';
import 'package:storykeep/api/api_client.dart';
import 'package:storykeep/api/token_store.dart';
import 'package:storykeep/state/auth_state.dart';

// Fixtures shaped exactly like the examples in docs/api.

Map<String, dynamic> userJson({int id = 1}) => {
      'id': id,
      'email': 'ada@example.com',
      'first_name': 'Ada',
      'last_name': 'Lovelace',
      'date_of_birth': '1990-12-10',
      'created_at': '2026-10-03T17:24:13Z',
      'updated_at': '2026-10-03T17:24:13Z',
    };

Map<String, dynamic> sessionJson() =>
    {'token': 'a' * 64, 'expires_at': '2026-11-02T17:24:13Z', 'user': userJson()};

Map<String, dynamic> profileJson({String role = 'OWNER', String type = 'CHILD'}) => {
      'id': 7,
      'profile_type': type,
      'name': 'Mira',
      'date_of_birth': '2024-03-01',
      'created_at': '2026-10-03T17:24:13Z',
      'updated_at': '2026-10-03T17:24:13Z',
      'role': role,
    };

Map<String, dynamic> relationshipJson({String role = 'OWNER', int id = 3}) => {
      'id': id,
      'profile_id': 7,
      'profile_name': 'Mira',
      'profile_type': 'CHILD',
      'relationship_type': 'PARENT_CHILD',
      'started_at': '2024-03-01',
      'ended_at': null,
      'created_at': '2026-10-03T17:24:13.088527Z',
      'role': role,
    };

Map<String, dynamic> memberJson({int userId = 2, String role = 'PARENT'}) => {
      'user_id': userId,
      'email': 'grace@example.com',
      'first_name': 'Grace',
      'last_name': 'Hopper',
      'role': role,
      'joined_at': '2026-10-03T17:24:13Z',
    };

Map<String, dynamic> memoryMediaJson() => {
      'id': 40,
      'media_type': 'IMAGE',
      'file_name': 'steps.jpg',
      'mime_type': 'image/jpeg',
      'file_size': 248113,
      'created_at': '2026-10-03T17:25:02Z',
      'content_url': '/api/media/40/content',
    };

Map<String, dynamic> memoryJson({int id = 12, int createdBy = 1, List<dynamic>? media}) => {
      'id': id,
      'relationship_id': 3,
      'category': 'MILESTONE',
      'title': 'First steps',
      'description': 'Across the living room',
      'memory_date': '2025-03-14',
      'created_by': {'id': createdBy, 'first_name': 'Ada', 'last_name': 'Lovelace'},
      'created_at': '2026-10-03T17:24:13Z',
      'updated_at': '2026-10-03T17:24:13Z',
      'tags': ['Home', 'walking'],
      'media': media ?? [],
    };

Map<String, dynamic> capsuleJson({String status = 'LOCKED', String unlockAt = '2042-03-01T08:00:00Z'}) => {
      'id': 12,
      'relationship_id': 3,
      'title': 'For your 18th birthday',
      'status': status,
      'unlock_at': unlockAt,
      'created_by': 2,
      'created_at': '2026-10-03T17:24:13Z',
      'updated_at': '2026-10-03T17:24:13Z',
      'media_count': 1,
    };

http.Response jsonResponse(Object? body, [int status = 200]) =>
    http.Response(jsonEncode(body), status, headers: {'content-type': 'application/json'});

http.Response errorResponse(int status, String code, String message) =>
    jsonResponse({'error': {'code': code, 'message': message}}, status);

/// An [ApiClient] backed by a [MockClient] that answers with [handler].
ApiClient fakeApi(Future<http.Response> Function(http.Request) handler, {String? token = 'tok'}) => ApiClient(
      baseUrl: Uri.parse('http://api.test'),
      tokenStore: MemoryTokenStore(token),
      httpClient: MockClient(handler),
    );

/// Pumps [child] inside the providers and MaterialApp the screens expect.
Future<void> pumpScreen(WidgetTester tester, Widget child, {required ApiClient api, User? user}) async {
  await api.restoreToken();
  final auth = AuthState(api)
    ..user = user
    ..status = AuthStatus.authenticated;
  await tester.pumpWidget(MultiProvider(
    providers: [
      Provider<ApiClient>.value(value: api),
      ChangeNotifierProvider<AuthState>.value(value: auth),
    ],
    child: MaterialApp(home: child),
  ));
  await tester.pumpAndSettle();
}
