import 'package:flutter_test/flutter_test.dart';
import 'package:storykeep/api/models.dart';

import 'support.dart';

void main() {
  test('User and Session', () {
    final s = Session.fromJson(sessionJson());
    expect(s.token.length, 64);
    expect(s.expiresAt, DateTime.utc(2026, 11, 2, 17, 24, 13));
    expect(s.user.email, 'ada@example.com');
    expect(s.user.dateOfBirth, DateTime(1990, 12, 10));
    expect(User.fromJson({...userJson(), 'date_of_birth': null}).dateOfBirth, isNull);
  });

  test('Profile, Relationship, CreatedProfile and Member', () {
    final p = Profile.fromJson(profileJson());
    expect(p.isChild, isTrue);
    expect(p.role, 'OWNER');

    final r = Relationship.fromJson(relationshipJson());
    expect(r.profileName, 'Mira');
    expect(r.startedAt, DateTime(2024, 3, 1));
    expect(r.endedAt, isNull);
    expect(r.createdAt.isUtc, isTrue);

    final c = CreatedProfile.fromJson({'profile': profileJson(), 'relationship': relationshipJson()});
    expect(c.relationship.id, 3);

    final m = Member.fromJson(memberJson());
    expect(m.fullName, 'Grace Hopper');
    expect(m.role, 'PARENT');
  });

  test('Memory with media, timeline page and tags', () {
    final m = Memory.fromJson(memoryJson(media: [memoryMediaJson()]));
    expect(m.memoryDate, DateTime(2025, 3, 14));
    expect(m.createdAt, DateTime.utc(2026, 10, 3, 17, 24, 13));
    expect(m.tags, ['Home', 'walking']);
    expect(m.media.single.isImage, isTrue);
    expect(m.media.single.uploadedBy, isNull);
    expect(m.createdBy.fullName, 'Ada Lovelace');
    final withUploader = Memory.fromJson(memoryJson(media: [
      {...memoryMediaJson(), 'uploaded_by': {'id': 2, 'first_name': 'Grace', 'last_name': 'Hopper'}},
    ]));
    expect(withUploader.media.single.uploadedBy!.id, 2);

    final page = TimelinePage.fromJson({'memories': [memoryJson()], 'total': 1, 'limit': 50, 'offset': 0});
    expect(page.hasMore, isFalse);

    final tag = Tag.fromJson({'name': 'Beach', 'memory_count': 3});
    expect(tag.memoryCount, 3);

    final input = MemoryInput(category: 'BIRTHDAY', title: 'Cake', memoryDate: DateTime(2025, 3, 1), tags: ['cake']);
    expect(input.toJson(), {
      'category': 'BIRTHDAY',
      'title': 'Cake',
      'description': null,
      'memory_date': '2025-03-01',
      'tags': ['cake'],
    });
  });

  test('Media object from the media endpoints', () {
    final m = MediaItem.fromJson({
      ...memoryMediaJson(),
      'uploaded_by': {'id': 2, 'first_name': 'Grace', 'last_name': 'Hopper'},
    });
    expect(m.uploadedBy!.firstName, 'Grace');
    expect(m.contentUrl, '/api/media/40/content');
    expect(m.fileSize, 248113);
  });

  test('Development record and observations', () {
    final r = DevelopmentRecord.fromJson({
      'id': 12,
      'profile_id': 7,
      'record_date': '2025-03-01',
      'notes': 'First birthday week',
      'created_by': {'id': 1, 'first_name': 'Ada', 'last_name': 'Lovelace'},
      'created_at': '2026-10-03T17:24:13Z',
      'observations': [
        {'id': 30, 'domain': 'MOTOR', 'observation': 'Took three steps unaided', 'created_at': '2026-10-03T17:24:13Z'},
        {'id': 31, 'domain': 'LANGUAGE', 'observation': 'Says "mama"', 'created_at': '2026-10-03T17:24:13Z'},
      ],
    });
    expect(r.recordDate, DateTime(2025, 3, 1));
    expect(r.observations.map((o) => o.domain), ['MOTOR', 'LANGUAGE']);

    final input = DevelopmentRecordInput(
      recordDate: DateTime(2025, 3, 1),
      observations: const [Observation(domain: 'MOTOR', observation: 'Walks')],
    );
    expect(input.toJson(), {
      'record_date': '2025-03-01',
      'notes': null,
      'observations': [
        {'domain': 'MOTOR', 'observation': 'Walks'},
      ],
    });
  });

  test('Measurement and chart series', () {
    final m = Measurement.fromJson({
      'id': 4,
      'profile_id': 7,
      'measurement_type': 'HEIGHT',
      'unit': 'cm',
      'value': 74.25,
      'measurement_date': '2025-03-01',
      'created_by': {'id': 1, 'first_name': 'Ada', 'last_name': 'Lovelace'},
      'created_at': '2026-10-03T17:24:13Z',
    });
    expect(m.value, 74.25);
    expect(m.unit, 'cm');

    final s = MeasurementSeries.fromJson({
      'measurement_type': 'HEIGHT',
      'unit': 'cm',
      'points': [
        {'date': '2025-01-15', 'value': 72},
        {'date': '2025-03-01', 'value': 76.125},
      ],
    });
    expect(s.points.first.value, 72.0);
    expect(s.points.last.date, DateTime(2025, 3, 1));
  });

  test('Capsule metadata and content', () {
    final locked = Capsule.fromJson(capsuleJson());
    expect(locked.isLocked, isTrue);
    expect(locked.unlockAt, DateTime.utc(2042, 3, 1, 8));
    expect(locked.createdBy, 2);
    expect(locked.mediaCount, 1);
    expect(locked.content, isNull);

    final opened = Capsule.fromJson({
      ...capsuleJson(status: 'OPENED'),
      'message': 'Happy birthday!',
      'media': [memoryMediaJson()],
    });
    expect(opened.content!.message, 'Happy birthday!');
    expect(opened.content!.media.single.id, 40);

    final openedEmpty = Capsule.fromJson({...capsuleJson(status: 'OPENED'), 'message': null, 'media': []});
    expect(openedEmpty.content, isNotNull);
    expect(openedEmpty.content!.message, isNull);
  });

  test('a non-opened capsule never carries content, even if the payload had some', () {
    for (final status in ['LOCKED', 'AVAILABLE', 'CANCELLED']) {
      final c = Capsule.fromJson({...capsuleJson(status: status), 'message': 'secret', 'media': [memoryMediaJson()]});
      expect(c.content, isNull, reason: status);
    }
  });

  test('Reference data', () {
    final r = ReferenceData.fromJson({
      'profile_types': ['CHILD', 'PET', 'PERSON', 'OTHER'],
      'relationship_types': ['PARENT_CHILD', 'OWNER_PET', 'FRIEND', 'FAMILY', 'PARTNER', 'OTHER'],
      'relationship_roles': ['OWNER', 'PARENT', 'MEMBER', 'VIEWER'],
      'memory_categories': ['GENERAL', 'MILESTONE'],
      'media_types': ['IMAGE', 'VIDEO', 'AUDIO', 'DOCUMENT'],
      'development_domains': ['PHYSICAL', 'MOTOR'],
      'measurement_types': [
        {'name': 'HEIGHT', 'unit': 'cm'},
        {'name': 'WEIGHT', 'unit': 'kg'},
      ],
      'time_capsule_statuses': ['LOCKED', 'AVAILABLE', 'OPENED', 'CANCELLED'],
    });
    expect(r.measurementUnits, {'HEIGHT': 'cm', 'WEIGHT': 'kg'});
    expect(r.relationshipRoles.first, 'OWNER');
  });

  group('TimelineFilter.toQueryParameters', () {
    test('defaults to newest first with paging only', () {
      expect(const TimelineFilter().toQueryParameters(), {'order': 'desc', 'limit': '20', 'offset': '0'});
    });

    test('omits blank search and tag, and supports ascending order', () {
      final q = const TimelineFilter(query: '   ', tag: ' ', ascending: true).toQueryParameters(limit: 5, offset: 10);
      expect(q, {'order': 'asc', 'limit': '5', 'offset': '10'});
    });

    test('copyWith can clear a field', () {
      final f = TimelineFilter(category: 'TRAVEL', from: DateTime(2025, 1, 1));
      final cleared = f.copyWith(category: () => null);
      expect(cleared.category, isNull);
      expect(cleared.from, DateTime(2025, 1, 1));
      expect(cleared.toQueryParameters()['from'], '2025-01-01');
    });
  });
}
