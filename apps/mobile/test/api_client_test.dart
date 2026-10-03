import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:storykeep/api/api_client.dart';
import 'package:storykeep/api/token_store.dart';

import 'support.dart';

void main() {
  group('authentication header', () {
    test('is attached to protected requests', () async {
      late http.Request seen;
      final api = fakeApi((req) async {
        seen = req;
        return jsonResponse([profileJson()]);
      });
      await api.restoreToken();
      final profiles = await api.listProfiles();
      expect(profiles.single.name, 'Mira');
      expect(seen.headers['Authorization'], 'Bearer tok');
      expect(seen.url.toString(), 'http://api.test/api/profiles');
    });

    test('is not sent on login, and the new token is stored and used afterwards', () async {
      final requests = <http.Request>[];
      final store = MemoryTokenStore();
      final api = ApiClient(
        baseUrl: Uri.parse('http://api.test/'),
        tokenStore: store,
        httpClient: MockClient((req) async {
          requests.add(req);
          return req.url.path == '/api/auth/login' ? jsonResponse(sessionJson()) : jsonResponse(userJson());
        }),
      );
      final session = await api.login('ada@example.com', 'correct horse');
      expect(requests.first.headers.containsKey('Authorization'), isFalse);
      expect(jsonDecode(requests.first.body), {'email': 'ada@example.com', 'password': 'correct horse'});
      expect(session.user.firstName, 'Ada');
      expect(store.token, 'a' * 64);

      await api.me();
      expect(requests.last.headers['Authorization'], 'Bearer ${'a' * 64}');
    });

    test('logout clears the token even if the server call fails', () async {
      final store = MemoryTokenStore('tok');
      final api = ApiClient(
        baseUrl: Uri.parse('http://api.test'),
        tokenStore: store,
        httpClient: MockClient((_) async => errorResponse(500, 'internal_error', 'boom')),
      );
      await api.restoreToken();
      await api.logout();
      expect(store.token, isNull);
      expect(api.hasToken, isFalse);
    });
  });

  group('errors', () {
    test('parses the documented error shape', () async {
      final api = fakeApi((_) async => errorResponse(409, 'conflict', 'already a member'));
      await api.restoreToken();
      await expectLater(
        api.addMember(3, email: 'grace@example.com', role: 'MEMBER'),
        throwsA(isA<ApiException>()
            .having((e) => e.status, 'status', 409)
            .having((e) => e.code, 'code', 'conflict')
            .having((e) => e.message, 'message', 'already a member')),
      );
    });

    test('tolerates non-JSON error bodies', () {
      final e = ApiClient.parseError(502, utf8.encode('<html>Bad gateway</html>'), 'Bad Gateway');
      expect(e.status, 502);
      expect(e.code, 'http_502');
      expect(e.message, 'Bad Gateway');
    });

    test('network failures become network_error', () async {
      final api = fakeApi((_) async => throw http.ClientException('Connection refused'));
      await expectLater(
        api.listProfiles(),
        throwsA(isA<ApiException>().having((e) => e.isNetworkError, 'isNetworkError', isTrue)),
      );
    });

    test('401 unauthorized clears the token and notifies', () async {
      final store = MemoryTokenStore('expired');
      var notified = 0;
      final api = ApiClient(
        baseUrl: Uri.parse('http://api.test'),
        tokenStore: store,
        httpClient: MockClient((_) async => errorResponse(401, 'unauthorized', 'missing or invalid token')),
      )..onUnauthorized = () => notified++;
      await api.restoreToken();
      await expectLater(api.listRelationships(), throwsA(isA<ApiException>()));
      expect(notified, 1);
      expect(store.token, isNull);
      expect(api.hasToken, isFalse);
      expect(api.authHeaders, isEmpty);
    });

    test('401 invalid_credentials on password change keeps the session', () async {
      final store = MemoryTokenStore('tok');
      var notified = 0;
      final api = ApiClient(
        baseUrl: Uri.parse('http://api.test'),
        tokenStore: store,
        httpClient: MockClient((_) async => errorResponse(401, 'invalid_credentials', 'wrong password')),
      )..onUnauthorized = () => notified++;
      await api.restoreToken();
      await expectLater(
        api.changePassword(currentPassword: 'nope', newPassword: 'whatever123'),
        throwsA(isA<ApiException>().having((e) => e.code, 'code', 'invalid_credentials')),
      );
      expect(notified, 0);
      expect(store.token, 'tok');
    });
  });

  group('requests', () {
    test('timeline filters become query parameters', () async {
      late Uri url;
      final api = fakeApi((req) async {
        url = req.url;
        return jsonResponse({'memories': [memoryJson()], 'total': 41, 'limit': 20, 'offset': 20});
      });
      await api.restoreToken();
      final page = await api.listMemories(
        3,
        filter: TimelineFilter(
          from: DateTime(2025, 1, 1),
          to: DateTime(2025, 12, 31),
          category: 'BIRTHDAY',
          tag: 'cake',
          query: '  party ',
        ),
        limit: 20,
        offset: 20,
      );
      expect(url.path, '/api/relationships/3/memories');
      expect(url.queryParameters, {
        'from': '2025-01-01',
        'to': '2025-12-31',
        'category': 'BIRTHDAY',
        'tag': 'cake',
        'q': 'party',
        'order': 'desc',
        'limit': '20',
        'offset': '20',
      });
      expect(page.total, 41);
      expect(page.hasMore, isTrue);
    });

    test('uploads send multipart parts named "file"', () async {
      late http.Request seen;
      final api = fakeApi((req) async {
        seen = req;
        return jsonResponse([memoryMediaJson()], 201);
      });
      await api.restoreToken();
      final media = await api.uploadMemoryMedia(42, [
        UploadFile.bytes([0xFF, 0xD8, 0xFF], filename: 'steps.jpg'),
        UploadFile.bytes(utf8.encode('%PDF-1.4'), filename: 'report.pdf'),
      ]);
      expect(media.single.id, 40);
      expect(seen.url.path, '/api/memories/42/media');
      expect(seen.headers['Authorization'], 'Bearer tok');
      expect(seen.headers['content-type'], startsWith('multipart/form-data; boundary='));
      final body = latin1.decode(seen.bodyBytes);
      expect('name="file"'.allMatches(body).length, 2);
      expect(body, contains('filename="steps.jpg"'));
      expect(body, contains('filename="report.pdf"'));
    });

    test('capsule update only sends the message when replacing it', () async {
      final bodies = <Map<String, dynamic>>[];
      final api = fakeApi((req) async {
        bodies.add(jsonDecode(req.body) as Map<String, dynamic>);
        return jsonResponse(capsuleJson());
      });
      await api.restoreToken();
      await api.updateCapsule(12, title: 'New title');
      await api.updateCapsule(12, replaceMessage: true, message: null);
      expect(bodies[0], {'title': 'New title'});
      expect(bodies[1], {'message': null});
    });

    test('media content URLs resolve against the base URL', () {
      final api = fakeApi((_) async => jsonResponse(null));
      expect(api.resolve('/api/media/40/content').toString(), 'http://api.test/api/media/40/content');
    });
  });
}
