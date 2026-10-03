import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:storykeep/api/api_client.dart';
import 'package:storykeep/ui/screens/members_tab.dart';
import 'package:storykeep/ui/screens/memory_screens.dart';
import 'package:storykeep/ui/screens/timeline_tab.dart';
import 'package:storykeep/util/permissions.dart';

import 'support.dart';

void main() {
  group('Perms', () {
    test('write and manage access follow the documented role table', () {
      expect(['OWNER', 'PARENT', 'MEMBER', 'VIEWER'].where(Perms.canWrite), ['OWNER', 'PARENT', 'MEMBER']);
      expect(['OWNER', 'PARENT', 'MEMBER', 'VIEWER'].where(Perms.canManage), ['OWNER', 'PARENT']);
    });

    test('items are editable by their creator or a manager', () {
      expect(Perms.canEditItem('MEMBER', createdBy: 1, currentUserId: 1), isTrue);
      expect(Perms.canEditItem('MEMBER', createdBy: 2, currentUserId: 1), isFalse);
      expect(Perms.canEditItem('VIEWER', createdBy: 1, currentUserId: 1), isFalse);
      expect(Perms.canEditItem('PARENT', createdBy: 2, currentUserId: 1), isTrue);
    });

    test('media can be removed by its uploader or a manager', () {
      expect(Perms.canRemoveMedia('MEMBER', uploadedBy: 1, currentUserId: 1), isTrue);
      expect(Perms.canRemoveMedia('MEMBER', uploadedBy: 2, currentUserId: 1), isFalse);
      expect(Perms.canRemoveMedia('VIEWER', uploadedBy: 1, currentUserId: 1), isFalse);
      expect(Perms.canRemoveMedia('PARENT', uploadedBy: 2, currentUserId: 1), isTrue);
      expect(Perms.canRemoveMedia('MEMBER', uploadedBy: null, currentUserId: 1, fallback: true), isTrue);
    });

    test('only owners grant or manage the OWNER role', () {
      expect(Perms.assignableRoles('PARENT'), isNot(contains('OWNER')));
      expect(Perms.assignableRoles('OWNER'), contains('OWNER'));
      expect(Perms.canManageMember('PARENT', 'OWNER'), isFalse);
      expect(Perms.canManageMember('PARENT', 'MEMBER'), isTrue);
      expect(Perms.canManageMember('OWNER', 'OWNER'), isTrue);
    });
  });

  Future<http.Response> backend(http.Request req, {String role = 'OWNER', int createdBy = 1}) async {
    final path = req.url.path;
    if (path == '/api/relationships/3/memories') {
      return jsonResponse({'memories': [memoryJson(createdBy: createdBy)], 'total': 1, 'limit': 20, 'offset': 0});
    }
    if (path == '/api/relationships/3/members') return jsonResponse([memberJson(userId: 1, role: role), memberJson()]);
    if (path == '/api/relationships/3') return jsonResponse(relationshipJson(role: role));
    if (path == '/api/memories/12') return jsonResponse(memoryJson(createdBy: createdBy));
    if (path.endsWith('/tags')) return jsonResponse([]);
    return errorResponse(404, 'not_found', 'not found');
  }

  final me = User.fromJson(userJson(id: 1));

  testWidgets('a VIEWER cannot add memories; a MEMBER can', (tester) async {
    for (final (role, expected) in [('VIEWER', findsNothing), ('MEMBER', findsOneWidget)]) {
      final api = fakeApi((r) => backend(r, role: role));
      await pumpScreen(
        tester,
        Scaffold(body: TimelineTab(key: ValueKey(role), relationship: Relationship.fromJson(relationshipJson(role: role)))),
        api: api,
        user: me,
      );
      expect(find.text('First steps'), findsOneWidget);
      expect(find.byKey(const Key('add-memory')), expected, reason: role);
    }
  });

  testWidgets('only managers see "Add member"', (tester) async {
    for (final (role, expected) in [('VIEWER', findsNothing), ('MEMBER', findsNothing), ('PARENT', findsOneWidget)]) {
      final api = fakeApi((r) => backend(r, role: role));
      await pumpScreen(
        tester,
        MembersTab(
          key: ValueKey(role),
          relationship: Relationship.fromJson(relationshipJson(role: role)),
          onRelationshipChanged: () async {},
        ),
        api: api,
        user: me,
      );
      expect(find.text('Grace Hopper'), findsOneWidget);
      expect(find.byKey(const Key('add-member')), expected, reason: role);
    }
  });

  testWidgets('a MEMBER cannot edit someone else\'s memory, but can edit their own', (tester) async {
    for (final (createdBy, role, expected) in [
      (2, 'MEMBER', findsNothing),
      (1, 'MEMBER', findsOneWidget),
      (2, 'PARENT', findsOneWidget),
      (1, 'VIEWER', findsNothing),
    ]) {
      final api = fakeApi((r) => backend(r, role: role, createdBy: createdBy));
      await pumpScreen(tester, MemoryDetailScreen(key: ValueKey('$role$createdBy'), memoryId: 12), api: api, user: me);
      expect(find.text('First steps'), findsOneWidget);
      expect(find.byKey(const Key('edit-memory')), expected, reason: '$role, creator $createdBy');
      expect(find.byKey(const Key('delete-memory')), expected, reason: '$role, creator $createdBy');
    }
  });

  testWidgets('the memory page shows event date and recorded date separately', (tester) async {
    final api = fakeApi((r) => backend(r));
    await pumpScreen(tester, const MemoryDetailScreen(memoryId: 12), api: api, user: me);
    expect(find.text('Mar 14, 2025'), findsOneWidget);
    expect(find.textContaining('by Ada Lovelace'), findsOneWidget);
    expect(find.text('Happened on: '), findsOneWidget);
    expect(find.text('Recorded: '), findsOneWidget);
  });
}
