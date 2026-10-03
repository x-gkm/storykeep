import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:http/http.dart' as http;
import 'package:http_parser/http_parser.dart';

import 'api_exception.dart';
import 'models.dart';
import 'token_store.dart';

export 'api_exception.dart';
export 'models.dart';

/// A file to upload as one `file` part of a multipart request.
class UploadFile {
  const UploadFile.path(String this.path, {required this.filename, this.contentType}) : bytes = null;
  const UploadFile.bytes(List<int> this.bytes, {required this.filename, this.contentType}) : path = null;

  final String filename;
  final String? path;
  final List<int>? bytes;

  /// Optional declared type; the server sniffs the content anyway. Only matters for
  /// MP4/WebM audio, which is stored as video unless declared as audio.
  final String? contentType;

  Future<http.MultipartFile> toPart() async {
    final type = contentType == null ? null : MediaType.parse(contentType!);
    if (path != null) {
      return http.MultipartFile.fromPath('file', path!, filename: filename, contentType: type);
    }
    return http.MultipartFile.fromBytes('file', bytes!, filename: filename, contentType: type);
  }
}

/// Typed client for the Storykeep REST API (see docs/api).
///
/// Holds the bearer token, persists it via [TokenStore] and attaches it to every
/// request. When the server answers `401 unauthorized` the token is cleared and
/// [onUnauthorized] is called so the app can return to the login screen.
class ApiClient {
  ApiClient({required Uri baseUrl, required TokenStore tokenStore, http.Client? httpClient})
      : baseUrl = _normalize(baseUrl),
        _tokens = tokenStore,
        _http = httpClient ?? http.Client();

  static const _jsonTimeout = Duration(seconds: 30);
  static const _uploadTimeout = Duration(minutes: 10);

  /// Server root, e.g. `http://10.0.2.2:3000` (API paths start with `/api`).
  final Uri baseUrl;
  final TokenStore _tokens;
  final http.Client _http;
  String? _token;

  /// Called after a request was rejected with `401 unauthorized` and the token cleared.
  void Function()? onUnauthorized;

  static Uri _normalize(Uri u) {
    var path = u.path;
    while (path.endsWith('/')) {
      path = path.substring(0, path.length - 1);
    }
    return u.replace(path: path);
  }

  /// Loads a previously stored token. Returns whether one exists.
  Future<bool> restoreToken() async {
    _token = await _tokens.read();
    return _token != null;
  }

  bool get hasToken => _token != null;

  /// Headers to authenticate requests made outside this client (e.g. `Image.network`).
  Map<String, String> get authHeaders => {if (_token != null) 'Authorization': 'Bearer $_token'};

  /// Resolves an API path such as `/api/media/4/content` against [baseUrl].
  Uri resolve(String path, [Map<String, String>? query]) {
    final p = path.startsWith('/') ? path : '/$path';
    return baseUrl.replace(path: '${baseUrl.path}$p', queryParameters: (query == null || query.isEmpty) ? null : query);
  }

  void close() => _http.close();

  // --- plumbing -------------------------------------------------------------

  Future<dynamic> _request(
    String method,
    String path, {
    Object? body,
    Map<String, String>? query,
    bool authenticated = true,
  }) async {
    final request = http.Request(method, resolve(path, query));
    request.headers['Accept'] = 'application/json';
    if (authenticated) request.headers.addAll(authHeaders);
    if (body != null) {
      request.headers['Content-Type'] = 'application/json';
      request.body = jsonEncode(body);
    }
    final response = await _send(request, _jsonTimeout);
    return _decode(response, authenticated: authenticated);
  }

  Future<http.Response> _send(http.BaseRequest request, Duration timeout) async {
    try {
      final streamed = await _http.send(request).timeout(timeout);
      return await http.Response.fromStream(streamed).timeout(timeout);
    } on TimeoutException {
      throw const ApiException(0, ApiException.networkErrorCode, 'The server took too long to respond.');
    } on http.ClientException catch (e) {
      throw ApiException(0, ApiException.networkErrorCode, 'Could not reach the server (${e.message}).');
    } on SocketException catch (e) {
      throw ApiException(0, ApiException.networkErrorCode, 'Could not reach the server (${e.message}).');
    }
  }

  Future<dynamic> _decode(http.Response response, {required bool authenticated}) async {
    final status = response.statusCode;
    if (status >= 200 && status < 300) {
      if (status == 204 || response.bodyBytes.isEmpty) return null;
      return jsonDecode(utf8.decode(response.bodyBytes));
    }
    final error = parseError(status, response.bodyBytes, response.reasonPhrase);
    if (authenticated && error.isUnauthorized) await _handleUnauthorized();
    throw error;
  }

  Future<void> _handleUnauthorized() async {
    final hadToken = _token != null;
    _token = null;
    await _tokens.delete();
    if (hadToken) onUnauthorized?.call();
  }

  /// Builds an [ApiException] from an error response body
  /// (`{"error": {"code": ..., "message": ...}}`), tolerating other bodies.
  static ApiException parseError(int status, List<int> body, [String? reason]) {
    try {
      final json = jsonDecode(utf8.decode(body));
      if (json is Map && json['error'] is Map) {
        final e = json['error'] as Map;
        return ApiException(status, '${e['code'] ?? 'http_$status'}', '${e['message'] ?? reason ?? 'Request failed'}');
      }
    } on FormatException {
      // Not JSON; fall through.
    }
    return ApiException(status, 'http_$status', reason?.isNotEmpty == true ? reason! : 'Request failed ($status)');
  }

  Future<Json> _json(String method, String path, {Object? body, Map<String, String>? query}) async =>
      await _request(method, path, body: body, query: query) as Json;

  Future<List<Json>> _list(String path, {Map<String, String>? query}) async =>
      ((await _request('GET', path, query: query)) as List).cast<Json>();

  Future<void> _setSession(Session s) async {
    _token = s.token;
    await _tokens.write(s.token);
  }

  // --- auth & users ---------------------------------------------------------

  Future<Session> register({
    required String email,
    required String password,
    required String firstName,
    required String lastName,
    DateTime? dateOfBirth,
  }) async {
    final json = await _request('POST', '/api/auth/register', authenticated: false, body: {
      'email': email,
      'password': password,
      'first_name': firstName,
      'last_name': lastName,
      'date_of_birth': dateOfBirth == null ? null : formatApiDate(dateOfBirth),
    });
    final session = Session.fromJson(json as Json);
    await _setSession(session);
    return session;
  }

  Future<Session> login(String email, String password) async {
    final json = await _request('POST', '/api/auth/login',
        authenticated: false, body: {'email': email, 'password': password});
    final session = Session.fromJson(json as Json);
    await _setSession(session);
    return session;
  }

  /// Ends the session on the server (best effort) and forgets the token.
  Future<void> logout() async {
    try {
      if (_token != null) await _request('POST', '/api/auth/logout');
    } on ApiException {
      // The token is dropped locally either way.
    } finally {
      _token = null;
      await _tokens.delete();
    }
  }

  Future<User> me() async => User.fromJson(await _json('GET', '/api/users/me'));

  Future<User> updateMe({required String firstName, required String lastName, DateTime? dateOfBirth}) async =>
      User.fromJson(await _json('PUT', '/api/users/me', body: {
        'first_name': firstName,
        'last_name': lastName,
        'date_of_birth': dateOfBirth == null ? null : formatApiDate(dateOfBirth),
      }));

  Future<void> changePassword({required String currentPassword, required String newPassword}) =>
      _request('PUT', '/api/users/me/password',
          body: {'current_password': currentPassword, 'new_password': newPassword});

  Future<ReferenceData> reference() async =>
      ReferenceData.fromJson(await _request('GET', '/api/reference', authenticated: false) as Json);

  // --- profiles ---------------------------------------------------------------

  Future<List<Profile>> listProfiles() async => (await _list('/api/profiles')).map(Profile.fromJson).toList();

  Future<CreatedProfile> createProfile({
    required String profileType,
    required String name,
    DateTime? dateOfBirth,
    required String relationshipType,
    DateTime? startedAt,
  }) async =>
      CreatedProfile.fromJson(await _json('POST', '/api/profiles', body: {
        'profile_type': profileType,
        'name': name,
        'date_of_birth': dateOfBirth == null ? null : formatApiDate(dateOfBirth),
        'relationship_type': relationshipType,
        'started_at': startedAt == null ? null : formatApiDate(startedAt),
      }));

  Future<Profile> getProfile(int id) async => Profile.fromJson(await _json('GET', '/api/profiles/$id'));

  Future<Profile> updateProfile(int id, {required String profileType, required String name, DateTime? dateOfBirth}) async =>
      Profile.fromJson(await _json('PUT', '/api/profiles/$id', body: {
        'profile_type': profileType,
        'name': name,
        'date_of_birth': dateOfBirth == null ? null : formatApiDate(dateOfBirth),
      }));

  Future<void> deleteProfile(int id) => _request('DELETE', '/api/profiles/$id');

  // --- relationships & members ----------------------------------------------

  Future<List<Relationship>> listRelationships() async =>
      (await _list('/api/relationships')).map(Relationship.fromJson).toList();

  Future<Relationship> createRelationship({
    required int profileId,
    required String relationshipType,
    DateTime? startedAt,
    DateTime? endedAt,
  }) async =>
      Relationship.fromJson(await _json('POST', '/api/relationships', body: {
        'profile_id': profileId,
        'relationship_type': relationshipType,
        'started_at': startedAt == null ? null : formatApiDate(startedAt),
        'ended_at': endedAt == null ? null : formatApiDate(endedAt),
      }));

  Future<Relationship> getRelationship(int id) async =>
      Relationship.fromJson(await _json('GET', '/api/relationships/$id'));

  Future<Relationship> updateRelationship(int id,
          {required String relationshipType, DateTime? startedAt, DateTime? endedAt}) async =>
      Relationship.fromJson(await _json('PUT', '/api/relationships/$id', body: {
        'relationship_type': relationshipType,
        'started_at': startedAt == null ? null : formatApiDate(startedAt),
        'ended_at': endedAt == null ? null : formatApiDate(endedAt),
      }));

  Future<void> deleteRelationship(int id) => _request('DELETE', '/api/relationships/$id');

  Future<List<Member>> listMembers(int relationshipId) async =>
      (await _list('/api/relationships/$relationshipId/members')).map(Member.fromJson).toList();

  Future<List<Member>> addMember(int relationshipId, {required String email, required String role}) async =>
      ((await _request('POST', '/api/relationships/$relationshipId/members', body: {'email': email, 'role': role}))
              as List)
          .map((m) => Member.fromJson(m as Json))
          .toList();

  Future<List<Member>> updateMemberRole(int relationshipId, int userId, String role) async =>
      ((await _request('PUT', '/api/relationships/$relationshipId/members/$userId', body: {'role': role})) as List)
          .map((m) => Member.fromJson(m as Json))
          .toList();

  /// Removes a member; with your own user id this leaves the relationship.
  Future<void> removeMember(int relationshipId, int userId) =>
      _request('DELETE', '/api/relationships/$relationshipId/members/$userId');

  // --- memories & tags ------------------------------------------------------

  Future<TimelinePage> listMemories(int relationshipId,
          {TimelineFilter filter = const TimelineFilter(), int limit = 20, int offset = 0}) async =>
      TimelinePage.fromJson(await _json('GET', '/api/relationships/$relationshipId/memories',
          query: filter.toQueryParameters(limit: limit, offset: offset)));

  Future<Memory> createMemory(int relationshipId, MemoryInput input) async =>
      Memory.fromJson(await _json('POST', '/api/relationships/$relationshipId/memories', body: input.toJson()));

  Future<Memory> getMemory(int id) async => Memory.fromJson(await _json('GET', '/api/memories/$id'));

  Future<Memory> updateMemory(int id, MemoryInput input) async =>
      Memory.fromJson(await _json('PUT', '/api/memories/$id', body: input.toJson()));

  Future<void> deleteMemory(int id) => _request('DELETE', '/api/memories/$id');

  Future<List<Tag>> listTags({String? query, int? limit}) async => (await _list('/api/tags', query: {
        if (query != null && query.trim().isNotEmpty) 'q': query.trim(),
        if (limit != null) 'limit': '$limit',
      }))
          .map(Tag.fromJson)
          .toList();

  Future<List<Tag>> listRelationshipTags(int relationshipId, {String? query, int? limit}) async =>
      (await _list('/api/relationships/$relationshipId/tags', query: {
        if (query != null && query.trim().isNotEmpty) 'q': query.trim(),
        if (limit != null) 'limit': '$limit',
      }))
          .map(Tag.fromJson)
          .toList();

  Future<Tag> createTag(String name) async => Tag.fromJson(await _json('POST', '/api/tags', body: {'name': name}));

  // --- media ----------------------------------------------------------------

  Future<List<MediaItem>> _upload(String path, List<UploadFile> files) async {
    final request = http.MultipartRequest('POST', resolve(path));
    request.headers['Accept'] = 'application/json';
    request.headers.addAll(authHeaders);
    for (final f in files) {
      request.files.add(await f.toPart());
    }
    final response = await _send(request, _uploadTimeout);
    final json = await _decode(response, authenticated: true) as List;
    return json.map((m) => MediaItem.fromJson(m as Json)).toList();
  }

  /// Uploads up to 20 files per request; larger batches are split.
  Future<List<MediaItem>> _uploadAll(String path, List<UploadFile> files) async {
    final result = <MediaItem>[];
    for (var i = 0; i < files.length; i += 20) {
      result.addAll(await _upload(path, files.sublist(i, i + 20 > files.length ? files.length : i + 20)));
    }
    return result;
  }

  Future<List<MediaItem>> uploadMemoryMedia(int memoryId, List<UploadFile> files) =>
      _uploadAll('/api/memories/$memoryId/media', files);

  Future<List<MediaItem>> uploadCapsuleMedia(int capsuleId, List<UploadFile> files) =>
      _uploadAll('/api/capsules/$capsuleId/media', files);

  Future<void> removeMemoryMedia(int memoryId, int mediaId) =>
      _request('DELETE', '/api/memories/$memoryId/media/$mediaId');

  Future<void> removeCapsuleMedia(int capsuleId, int mediaId) =>
      _request('DELETE', '/api/capsules/$capsuleId/media/$mediaId');

  Future<MediaItem> getMedia(int id) async => MediaItem.fromJson(await _json('GET', '/api/media/$id'));

  /// Downloads a media file's bytes from its `content_url`.
  Future<Uint8List> downloadMedia(String contentUrl) async {
    final request = http.Request('GET', resolve(contentUrl))..headers.addAll(authHeaders);
    final response = await _send(request, _uploadTimeout);
    if (response.statusCode != 200) await _decode(response, authenticated: true);
    return response.bodyBytes;
  }

  /// Downloads a media file into [file], streaming it to disk.
  Future<File> downloadMediaTo(String contentUrl, File file) async {
    final request = http.Request('GET', resolve(contentUrl))..headers.addAll(authHeaders);
    final http.StreamedResponse streamed;
    try {
      streamed = await _http.send(request).timeout(_jsonTimeout);
    } on Exception catch (e) {
      throw ApiException(0, ApiException.networkErrorCode, 'Could not reach the server ($e).');
    }
    if (streamed.statusCode != 200) {
      await _decode(await http.Response.fromStream(streamed), authenticated: true);
    }
    final sink = file.openWrite();
    try {
      await streamed.stream.pipe(sink);
    } catch (_) {
      await sink.close();
      rethrow;
    }
    return file;
  }

  // --- development & measurements -------------------------------------------

  Future<List<DevelopmentRecord>> listDevelopmentRecords(int profileId,
          {DateTime? from, DateTime? to, String? domain}) async =>
      (await _list('/api/profiles/$profileId/development-records', query: {
        if (from != null) 'from': formatApiDate(from),
        if (to != null) 'to': formatApiDate(to),
        'domain': ?domain,
      }))
          .map(DevelopmentRecord.fromJson)
          .toList();

  Future<DevelopmentRecord> createDevelopmentRecord(int profileId, DevelopmentRecordInput input) async =>
      DevelopmentRecord.fromJson(
          await _json('POST', '/api/profiles/$profileId/development-records', body: input.toJson()));

  Future<DevelopmentRecord> getDevelopmentRecord(int id) async =>
      DevelopmentRecord.fromJson(await _json('GET', '/api/development-records/$id'));

  Future<DevelopmentRecord> updateDevelopmentRecord(int id, DevelopmentRecordInput input) async =>
      DevelopmentRecord.fromJson(await _json('PUT', '/api/development-records/$id', body: input.toJson()));

  Future<void> deleteDevelopmentRecord(int id) => _request('DELETE', '/api/development-records/$id');

  Map<String, String> _measurementQuery(String? type, DateTime? from, DateTime? to) => {
        'type': ?type,
        if (from != null) 'from': formatApiDate(from),
        if (to != null) 'to': formatApiDate(to),
      };

  Future<List<Measurement>> listMeasurements(int profileId, {String? type, DateTime? from, DateTime? to}) async =>
      (await _list('/api/profiles/$profileId/measurements', query: _measurementQuery(type, from, to)))
          .map(Measurement.fromJson)
          .toList();

  Future<Measurement> createMeasurement(int profileId, MeasurementInput input) async =>
      Measurement.fromJson(await _json('POST', '/api/profiles/$profileId/measurements', body: input.toJson()));

  Future<Measurement> getMeasurement(int id) async => Measurement.fromJson(await _json('GET', '/api/measurements/$id'));

  Future<Measurement> updateMeasurement(int id, MeasurementInput input) async =>
      Measurement.fromJson(await _json('PUT', '/api/measurements/$id', body: input.toJson()));

  Future<void> deleteMeasurement(int id) => _request('DELETE', '/api/measurements/$id');

  /// One chart series for [type].
  Future<MeasurementSeries> measurementSeries(int profileId, String type, {DateTime? from, DateTime? to}) async =>
      MeasurementSeries.fromJson(await _json('GET', '/api/profiles/$profileId/measurements/series',
          query: _measurementQuery(type, from, to)));

  /// Every measurement type that has data, one series each.
  Future<List<MeasurementSeries>> allMeasurementSeries(int profileId, {DateTime? from, DateTime? to}) async =>
      (await _list('/api/profiles/$profileId/measurements/series', query: _measurementQuery(null, from, to)))
          .map(MeasurementSeries.fromJson)
          .toList();

  // --- time capsules --------------------------------------------------------

  Future<List<Capsule>> listCapsules(int relationshipId, {String? status}) async =>
      (await _list('/api/relationships/$relationshipId/capsules', query: {'status': ?status}))
          .map(Capsule.fromJson)
          .toList();

  Future<Capsule> createCapsule(int relationshipId,
          {required String title, String? message, required DateTime unlockAt}) async =>
      Capsule.fromJson(await _json('POST', '/api/relationships/$relationshipId/capsules', body: {
        'title': title,
        'message': message,
        'unlock_at': unlockAt.toUtc().toIso8601String(),
      }));

  Future<Capsule> getCapsule(int id) async => Capsule.fromJson(await _json('GET', '/api/capsules/$id'));

  /// Partial update. The sealed message can't be read back, so it is only sent when
  /// [replaceMessage] is true; a null or blank [message] then clears it.
  Future<Capsule> updateCapsule(int id,
          {String? title, DateTime? unlockAt, bool replaceMessage = false, String? message}) async =>
      Capsule.fromJson(await _json('PUT', '/api/capsules/$id', body: {
        'title': ?title,
        if (unlockAt != null) 'unlock_at': unlockAt.toUtc().toIso8601String(),
        if (replaceMessage) 'message': message,
      }));

  Future<Capsule> cancelCapsule(int id) async => Capsule.fromJson(await _json('POST', '/api/capsules/$id/cancel'));

  Future<Capsule> openCapsule(int id) async => Capsule.fromJson(await _json('POST', '/api/capsules/$id/open'));

  Future<void> deleteCapsule(int id) => _request('DELETE', '/api/capsules/$id');
}
