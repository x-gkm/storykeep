import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import 'api/api_client.dart';
import 'state/auth_state.dart';
import 'ui/screens/account_screen.dart';
import 'ui/screens/auth_screens.dart';
import 'ui/screens/capsule_screens.dart';
import 'ui/screens/create_profile_screen.dart';
import 'ui/screens/dashboard_screen.dart';
import 'ui/screens/development_screens.dart';
import 'ui/screens/memory_screens.dart';
import 'ui/screens/profile_screen.dart';
import 'ui/screens/relationship_screen.dart';

const _seed = Color(0xFF3F6F73);

ThemeData buildTheme(Brightness brightness) {
  final scheme = ColorScheme.fromSeed(seedColor: _seed, brightness: brightness);
  return ThemeData(
    colorScheme: scheme,
    useMaterial3: true,
    inputDecorationTheme: const InputDecorationTheme(border: OutlineInputBorder()),
    cardTheme: const CardThemeData(margin: EdgeInsets.zero),
    appBarTheme: const AppBarTheme(centerTitle: false),
  );
}

int _id(GoRouterState s, String name) => int.parse(s.pathParameters[name]!);

GoRouter createRouter(AuthState auth) {
  const publicPaths = {'/login', '/register'};
  return GoRouter(
    initialLocation: '/',
    refreshListenable: auth,
    redirect: (context, state) {
      final path = state.matchedLocation;
      switch (auth.status) {
        case AuthStatus.unknown:
          return path == '/splash' ? null : '/splash';
        case AuthStatus.unauthenticated:
          return publicPaths.contains(path) ? null : '/login';
        case AuthStatus.authenticated:
          return (publicPaths.contains(path) || path == '/splash') ? '/' : null;
      }
    },
    routes: [
      GoRoute(path: '/splash', builder: (_, _) => const SplashScreen()),
      GoRoute(path: '/login', builder: (_, _) => const LoginScreen()),
      GoRoute(path: '/register', builder: (_, _) => const RegisterScreen()),
      GoRoute(path: '/', builder: (_, _) => const DashboardScreen()),
      GoRoute(path: '/account', builder: (_, _) => const AccountScreen()),
      GoRoute(path: '/profiles/new', builder: (_, _) => const CreateProfileScreen()),
      GoRoute(path: '/profiles/:pid', builder: (_, s) => ProfileScreen(profileId: _id(s, 'pid'))),
      GoRoute(
        path: '/profiles/:pid/development/new',
        builder: (_, s) => DevelopmentRecordFormScreen(profileId: _id(s, 'pid')),
      ),
      GoRoute(
        path: '/development-records/:id/edit',
        builder: (_, s) => DevelopmentRecordFormScreen(recordId: _id(s, 'id')),
      ),
      GoRoute(
        path: '/relationships/:rid',
        builder: (_, s) => RelationshipScreen(
          relationshipId: _id(s, 'rid'),
          initialTab: int.tryParse(s.uri.queryParameters['tab'] ?? '') ?? 0,
        ),
      ),
      GoRoute(
        path: '/relationships/:rid/memories/new',
        builder: (_, s) => MemoryFormScreen(relationshipId: _id(s, 'rid')),
      ),
      GoRoute(path: '/memories/:mid', builder: (_, s) => MemoryDetailScreen(memoryId: _id(s, 'mid'))),
      GoRoute(path: '/memories/:mid/edit', builder: (_, s) => MemoryFormScreen(memoryId: _id(s, 'mid'))),
      GoRoute(
        path: '/relationships/:rid/capsules/new',
        builder: (_, s) => CapsuleFormScreen(relationshipId: _id(s, 'rid')),
      ),
      GoRoute(path: '/capsules/:cid', builder: (_, s) => CapsuleDetailScreen(capsuleId: _id(s, 'cid'))),
      GoRoute(path: '/capsules/:cid/edit', builder: (_, s) => CapsuleFormScreen(capsuleId: _id(s, 'cid'))),
    ],
  );
}

class StorykeepApp extends StatefulWidget {
  const StorykeepApp({super.key, required this.api, required this.auth});

  final ApiClient api;
  final AuthState auth;

  @override
  State<StorykeepApp> createState() => _StorykeepAppState();
}

class _StorykeepAppState extends State<StorykeepApp> {
  late final GoRouter _router = createRouter(widget.auth);

  @override
  void dispose() {
    _router.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: widget.api),
        ChangeNotifierProvider<AuthState>.value(value: widget.auth),
      ],
      child: MaterialApp.router(
        title: 'Storykeep',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        routerConfig: _router,
      ),
    );
  }
}
