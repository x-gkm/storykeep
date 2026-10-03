// Smoke test of the real Dart API client against a running backend.
//
// Skipped unless STORYKEEP_E2E_URL is set, e.g.
//   STORYKEEP_E2E_URL=http://127.0.0.1:3102 flutter test test/integration
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:storykeep/api/api_client.dart';
import 'package:storykeep/api/token_store.dart';

final _url = Platform.environment['STORYKEEP_E2E_URL'];

/// A valid 1×1 PNG.
final _png = base64Decode(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==');

final _pdf = utf8.encode('%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n');

ApiClient _client() => ApiClient(baseUrl: Uri.parse(_url!), tokenStore: MemoryTokenStore());

Matcher _apiError(int status) => throwsA(isA<ApiException>().having((e) => e.status, 'status', status));

void main() {
  setUpAll(() {
    // flutter_test may install HttpOverrides that block real network access.
    HttpOverrides.global = null;
  });

  test('register → profile → memory with media → capsule → measurements', () async {
    final stamp = DateTime.now().microsecondsSinceEpoch;
    final api = _client();
    var unauthorizedCalls = 0;
    api.onUnauthorized = () => unauthorizedCalls++;

    // Auth and account.
    final session = await api.register(
      email: 'Ada.$stamp@Example.com',
      password: 'correct horse battery',
      firstName: 'Ada',
      lastName: 'Lovelace',
      dateOfBirth: DateTime(1990, 12, 10),
    );
    expect(session.user.email, 'ada.$stamp@example.com');
    expect((await api.me()).id, session.user.id);
    final renamed = await api.updateMe(firstName: 'Ada', lastName: 'King', dateOfBirth: null);
    expect(renamed.lastName, 'King');
    expect(renamed.dateOfBirth, isNull);
    final reference = await api.reference();
    expect(reference.measurementUnits['WEIGHT'], 'kg');

    // Profile + first relationship in one call.
    final created = await api.createProfile(
      profileType: 'CHILD',
      name: 'Mira',
      dateOfBirth: DateTime(2024, 3, 1),
      relationshipType: 'PARENT_CHILD',
      startedAt: DateTime(2024, 3, 1),
    );
    expect(created.profile.role, 'OWNER');
    expect(created.relationship.role, 'OWNER');
    final relId = created.relationship.id;
    final profileId = created.profile.id;
    expect((await api.listRelationships()).map((r) => r.id), contains(relId));
    expect((await api.listProfiles()).single.name, 'Mira');
    final family = await api.createRelationship(profileId: profileId, relationshipType: 'FAMILY');
    await api.updateRelationship(family.id, relationshipType: 'FAMILY', startedAt: DateTime(2024, 4, 1));
    await api.deleteRelationship(family.id);

    // Memory with tags, then the timeline filters.
    final memory = await api.createMemory(
      relId,
      MemoryInput(
        category: 'MILESTONE',
        title: 'First steps',
        description: 'Across the living room',
        memoryDate: DateTime(2025, 3, 14),
        tags: ['walking', 'Home', 'home'],
      ),
    );
    expect(memory.tags, ['Home', 'walking']);
    await api.createMemory(relId, MemoryInput(category: 'BIRTHDAY', title: 'First birthday', memoryDate: DateTime(2025, 3, 1)));

    var page = await api.listMemories(relId);
    expect(page.total, 2);
    expect(page.memories.first.title, 'First steps'); // newest first
    page = await api.listMemories(relId, filter: const TimelineFilter(tag: 'WALKING'));
    expect(page.memories.single.id, memory.id);
    page = await api.listMemories(relId, filter: const TimelineFilter(category: 'BIRTHDAY'));
    expect(page.memories.single.title, 'First birthday');
    page = await api.listMemories(relId, filter: const TimelineFilter(query: 'living'));
    expect(page.total, 1);
    page = await api.listMemories(relId, filter: TimelineFilter(from: DateTime(2025, 3, 2), to: DateTime(2025, 12, 31)));
    expect(page.memories.single.id, memory.id);
    page = await api.listMemories(relId, filter: const TimelineFilter(ascending: true), limit: 1, offset: 0);
    expect(page.memories.single.title, 'First birthday');
    expect(page.hasMore, isTrue);
    expect((await api.listRelationshipTags(relId)).map((t) => t.name), containsAll(['Home', 'walking']));
    expect((await api.listTags(query: 'walk')).single.name, 'walking');
    await expectLater(
        api.listMemories(relId, filter: TimelineFilter(from: DateTime(2025, 2, 1), to: DateTime(2025, 1, 1))),
        _apiError(400));

    // Media upload, metadata and authenticated download.
    final media = await api.uploadMemoryMedia(memory.id, [
      UploadFile.bytes(_png, filename: 'steps.png'),
      UploadFile.bytes(_pdf, filename: 'notes.pdf'),
    ]);
    expect(media.map((m) => m.mediaType), ['IMAGE', 'DOCUMENT']);
    expect(media.first.mimeType, 'image/png');
    final withMedia = await api.getMemory(memory.id);
    expect(withMedia.media.length, 2);
    expect(await api.downloadMedia(withMedia.media.first.contentUrl), _png);
    final tmp = File('${Directory.systemTemp.path}/storykeep-e2e-$stamp.pdf');
    await api.downloadMediaTo(withMedia.media.last.contentUrl, tmp);
    expect(await tmp.readAsBytes(), _pdf);
    await tmp.delete();
    expect((await api.getMedia(media.first.id)).uploadedBy!.id, session.user.id);
    await expectLater(api.uploadMemoryMedia(memory.id, [UploadFile.bytes(utf8.encode('<svg/>'), filename: 'x.svg')]),
        _apiError(400));
    await api.removeMemoryMedia(memory.id, media.last.id);
    expect((await api.getMemory(memory.id)).media.length, 1);

    // Edit the memory.
    final edited = await api.updateMemory(
      memory.id,
      MemoryInput(category: 'FIRST_TIME', title: 'First real steps', memoryDate: DateTime(2025, 3, 15), tags: ['walking']),
    );
    expect(edited.title, 'First real steps');
    expect(edited.description, isNull);
    expect(edited.media.length, 1);

    // Time capsules: sealed content stays hidden, opening works once unlocked.
    final far = await api.createCapsule(relId,
        title: 'For your 18th', message: 'Happy 18th!', unlockAt: DateTime.now().add(const Duration(days: 365)));
    expect(far.isLocked, isTrue);
    expect(far.content, isNull);
    final sealed = await api.uploadCapsuleMedia(far.id, [UploadFile.bytes(_png, filename: 'sealed.png')]);
    final fetched = await api.getCapsule(far.id);
    expect(fetched.mediaCount, 1);
    expect(fetched.content, isNull);
    await expectLater(api.openCapsule(far.id), _apiError(409));
    await expectLater(api.getMedia(sealed.single.id), _apiError(404));
    await expectLater(api.downloadMedia(sealed.single.contentUrl), _apiError(404));
    final retitled = await api.updateCapsule(far.id, title: 'For your 18th birthday');
    expect(retitled.title, 'For your 18th birthday');
    final cancelled = await api.cancelCapsule(far.id);
    expect(cancelled.isCancelled, isTrue);
    await expectLater(api.updateCapsule(far.id, title: 'x'), _apiError(409));

    final soon = await api.createCapsule(relId,
        title: 'Soon', message: 'Opened!', unlockAt: DateTime.now().add(const Duration(seconds: 3)));
    await api.uploadCapsuleMedia(soon.id, [UploadFile.bytes(_png, filename: 'soon.png')]);
    await Future<void>.delayed(const Duration(seconds: 5));
    expect((await api.getCapsule(soon.id)).isAvailable, isTrue);
    final opened = await api.openCapsule(soon.id);
    expect(opened.isOpened, isTrue);
    expect(opened.content!.message, 'Opened!');
    expect(await api.downloadMedia(opened.content!.media.single.contentUrl), _png);
    expect((await api.listCapsules(relId, status: 'OPENED')).single.id, soon.id);
    expect((await api.listCapsules(relId)).length, 2);
    await api.deleteCapsule(soon.id);

    // Measurements and the chart series.
    final h1 = await api.createMeasurement(
        profileId, MeasurementInput(measurementType: 'HEIGHT', value: 72, measurementDate: DateTime(2025, 1, 15)));
    await api.createMeasurement(
        profileId, MeasurementInput(measurementType: 'HEIGHT', value: 76.125, measurementDate: DateTime(2025, 3, 1)));
    await api.createMeasurement(
        profileId, MeasurementInput(measurementType: 'WEIGHT', value: 9.5, measurementDate: DateTime(2025, 3, 1)));
    final series = await api.measurementSeries(profileId, 'HEIGHT');
    expect(series.unit, 'cm');
    expect(series.points.map((p) => p.value), [72.0, 76.125]);
    expect((await api.allMeasurementSeries(profileId)).map((s) => s.measurementType), ['HEIGHT', 'WEIGHT']);
    expect((await api.listMeasurements(profileId, type: 'WEIGHT')).single.value, 9.5);
    final updated = await api.updateMeasurement(
        h1.id, MeasurementInput(measurementType: 'HEIGHT', value: 72.5, measurementDate: DateTime(2025, 1, 15)));
    expect(updated.value, 72.5);
    await expectLater(
        api.createMeasurement(
            profileId, MeasurementInput(measurementType: 'HEIGHT', value: 50, measurementDate: DateTime(2023, 1, 1))),
        _apiError(400));

    // Development records.
    final record = await api.createDevelopmentRecord(
      profileId,
      DevelopmentRecordInput(recordDate: DateTime(2025, 3, 1), notes: 'Birthday week', observations: const [
        Observation(domain: 'MOTOR', observation: 'Took three steps'),
        Observation(domain: 'LANGUAGE', observation: 'Says "mama"'),
      ]),
    );
    expect(record.observations.length, 2);
    expect((await api.listDevelopmentRecords(profileId, domain: 'LANGUAGE')).single.id, record.id);
    expect(await api.listDevelopmentRecords(profileId, domain: 'COGNITIVE'), isEmpty);
    await api.updateDevelopmentRecord(record.id, DevelopmentRecordInput(recordDate: DateTime(2025, 3, 2), notes: 'Edited'));
    expect((await api.getDevelopmentRecord(record.id)).observations, isEmpty);

    // A second user as VIEWER: can read, can't write.
    final viewer = _client();
    final viewerSession = await viewer.register(
        email: 'grace.$stamp@example.com', password: 'another password', firstName: 'Grace', lastName: 'Hopper');
    await expectLater(viewer.getRelationship(relId), _apiError(404));
    final members = await api.addMember(relId, email: 'grace.$stamp@example.com', role: 'VIEWER');
    expect(members.map((m) => m.role), ['OWNER', 'VIEWER']);
    expect((await viewer.getRelationship(relId)).role, 'VIEWER');
    expect((await viewer.listMemories(relId)).total, 2);
    expect(await viewer.downloadMedia(withMedia.media.first.contentUrl), _png);
    await expectLater(
        viewer.createMemory(relId, MemoryInput(category: 'GENERAL', title: 'x', memoryDate: DateTime(2025, 1, 1))),
        throwsA(isA<ApiException>().having((e) => e.code, 'code', 'forbidden')));
    await expectLater(api.addMember(relId, email: 'grace.$stamp@example.com', role: 'MEMBER'), _apiError(409));
    await api.updateMemberRole(relId, viewerSession.user.id, 'MEMBER');
    expect((await viewer.listMembers(relId)).length, 2);
    await expectLater(api.removeMember(relId, session.user.id), _apiError(409)); // last owner
    await viewer.removeMember(relId, viewerSession.user.id); // leave
    await expectLater(viewer.getRelationship(relId), _apiError(404));

    // Password change and session handling.
    await expectLater(api.changePassword(currentPassword: 'wrong password', newPassword: 'brand new password'),
        throwsA(isA<ApiException>().having((e) => e.code, 'code', 'invalid_credentials')));
    expect(unauthorizedCalls, 0);
    await api.changePassword(currentPassword: 'correct horse battery', newPassword: 'brand new password');
    await api.logout();
    expect(api.hasToken, isFalse);
    await api.login('ada.$stamp@example.com', 'brand new password');
    expect((await api.me()).lastName, 'King');

    // Delete the memory, then the whole profile.
    await api.deleteMemory(memory.id);
    await expectLater(api.getMemory(memory.id), _apiError(404));
    await api.deleteProfile(profileId);
    expect(await api.listProfiles(), isEmpty);

    // An invalid token triggers the 401 path.
    final stale = ApiClient(baseUrl: Uri.parse(_url!), tokenStore: MemoryTokenStore('0' * 64));
    var staleNotified = false;
    stale.onUnauthorized = () => staleNotified = true;
    await stale.restoreToken();
    await expectLater(stale.me(), _apiError(401));
    expect(staleNotified, isTrue);
    expect(stale.hasToken, isFalse);
  }, skip: _url == null ? 'set STORYKEEP_E2E_URL to run against a live backend' : false, timeout: const Timeout(Duration(minutes: 2)));
}
