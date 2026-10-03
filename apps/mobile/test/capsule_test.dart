import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:storykeep/api/api_client.dart';
import 'package:storykeep/state/auth_state.dart';
import 'package:storykeep/ui/screens/capsule_screens.dart';

import 'support.dart';

void main() {
  const secret = 'TOP SECRET MESSAGE';

  Future<void> pumpBody(WidgetTester tester, Capsule capsule) async {
    final api = fakeApi((_) async => jsonResponse(null));
    await tester.pumpWidget(Provider<ApiClient>.value(
      value: api,
      child: MaterialApp(home: Scaffold(body: CapsuleBody(capsule: capsule))),
    ));
    await tester.pump();
  }

  Future<void> tearDownTimers(WidgetTester tester) => tester.pumpWidget(const SizedBox());

  testWidgets('a locked capsule shows a countdown and never its content', (tester) async {
    final unlockAt = DateTime.now().toUtc().add(const Duration(days: 3, hours: 2)).toIso8601String();
    // Even if a payload smuggled content in, a LOCKED capsule must not show it.
    final capsule = Capsule.fromJson({...capsuleJson(unlockAt: unlockAt), 'message': secret, 'media': [memoryMediaJson()]});
    await pumpBody(tester, capsule);

    expect(find.text(secret), findsNothing);
    expect(find.byKey(const Key('capsule-message')), findsNothing);
    expect(find.text('steps.jpg'), findsNothing);
    expect(find.byKey(const Key('capsule-countdown')), findsOneWidget);
    expect(find.textContaining('3d 0'), findsOneWidget);
    expect(find.byKey(const Key('open-capsule')), findsNothing);

    // Still sealed after the countdown ticks.
    await tester.pump(const Duration(seconds: 2));
    expect(find.text(secret), findsNothing);
    await tearDownTimers(tester);
  });

  testWidgets('an available capsule offers to open it but shows no content', (tester) async {
    final capsule = Capsule.fromJson({...capsuleJson(status: 'AVAILABLE', unlockAt: '2020-01-01T00:00:00Z'), 'message': secret});
    await pumpBody(tester, capsule);
    expect(find.byKey(const Key('open-capsule')), findsOneWidget);
    expect(find.text(secret), findsNothing);
    await tearDownTimers(tester);
  });

  testWidgets('a cancelled capsule never shows content', (tester) async {
    final capsule = Capsule.fromJson({...capsuleJson(status: 'CANCELLED'), 'message': secret});
    await pumpBody(tester, capsule);
    expect(find.text(secret), findsNothing);
    expect(find.text('This capsule was cancelled'), findsOneWidget);
    await tearDownTimers(tester);
  });

  testWidgets('an opened capsule shows its message', (tester) async {
    final capsule = Capsule.fromJson({...capsuleJson(status: 'OPENED', unlockAt: '2020-01-01T00:00:00Z'), 'message': secret, 'media': []});
    await pumpBody(tester, capsule);
    expect(find.text(secret), findsOneWidget);
    await tearDownTimers(tester);
  });

  testWidgets('the detail screen of a locked capsule hides content from the API payload', (tester) async {
    final unlockAt = DateTime.now().toUtc().add(const Duration(days: 30)).toIso8601String();
    final api = fakeApi((req) async {
      if (req.url.path == '/api/capsules/12') {
        return jsonResponse({...capsuleJson(unlockAt: unlockAt), 'message': secret});
      }
      return jsonResponse(relationshipJson(role: 'VIEWER'));
    });
    await api.restoreToken();
    final auth = AuthState(api)
      ..user = User.fromJson(userJson())
      ..status = AuthStatus.authenticated;
    await tester.pumpWidget(MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: api),
        ChangeNotifierProvider<AuthState>.value(value: auth),
      ],
      child: const MaterialApp(home: CapsuleDetailScreen(capsuleId: 12)),
    ));
    await tester.pump();
    await tester.pump();
    await tester.pump();

    expect(find.text('For your 18th birthday'), findsOneWidget);
    expect(find.text(secret), findsNothing);
    // A VIEWER who didn't create it gets no edit or attach actions.
    expect(find.byIcon(Icons.edit_outlined), findsNothing);
    expect(find.text('Gallery'), findsNothing);
    await tearDownTimers(tester);
  });
}
