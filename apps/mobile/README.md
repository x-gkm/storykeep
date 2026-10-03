# Storykeep mobile

Flutter app (Android + iOS) for Storykeep, built against the REST API documented in
[`docs/api`](../../docs/api/README.md).

## Setup

Flutter, the Android SDK and a JDK come from the repository's Nix devshell:

```sh
nix develop            # from the repository root
cd apps/mobile
flutter pub get
```

Start the backend (see the root README); by default it listens on `127.0.0.1:3000`.

## Running

```sh
# Android emulator: the host machine is 10.0.2.2, which is the default
flutter run

# Physical device or another server: point the app at it
flutter run --dart-define=API_BASE_URL=http://192.168.1.20:3000
```

`API_BASE_URL` is the server root, **without** `/api`. Without it the app uses
`http://10.0.2.2:3000` on Android and `http://127.0.0.1:3000` on iOS (simulator).

### Linux desktop

The same app also builds as a Linux desktop app, which is the quickest way to try it without a phone. It talks to `http://127.0.0.1:3000` by default:

```sh
flutter run -d linux
# or build once and run the binary
flutter build linux --debug && build/linux/x64/debug/bundle/storykeep
```

On Linux there's no camera capture (the Photo/Video buttons are hidden), and the sign-in token is stored with the desktop keyring (Secret Service: GNOME Keyring or KWallet). Without a running keyring the app keeps the token in memory only, so you sign in again on each launch.

Plain HTTP is allowed only for development: the Android **debug** manifest sets
`usesCleartextTraffic` (release builds keep Android's default of blocking it), and iOS
allows local networking only (`NSAllowsLocalNetworking`). Use HTTPS in production.

## Checks

```sh
flutter analyze
flutter test                      # unit and widget tests (mocked HTTP)
flutter build apk --debug
```

### Smoke test against a real backend

`test/integration/e2e_test.dart` drives the real API client through register → profile →
memory with image/PDF upload and download → time capsules (locked, cancelled, opened) →
measurements and chart series → development records → a second member with the VIEWER role →
password change → 401 handling. It is skipped unless `STORYKEEP_E2E_URL` is set:

```sh
# with a backend on a throwaway database, e.g. port 3102
STORYKEEP_E2E_URL=http://127.0.0.1:3102 flutter test test/integration
```

## Architecture

```
lib/
  main.dart, app.dart        bootstrap, Material 3 theme, go_router routes + auth redirect
  config.dart                API_BASE_URL
  api/
    api_client.dart          one method per endpoint; bearer token; ApiException(status, code, message)
    models.dart              DTOs mirroring docs/api (fromJson / toJson), TimelineFilter
    token_store.dart         TokenStore interface (+ in-memory one for tests)
    secure_token_store.dart  flutter_secure_storage (Keystore / Keychain)
  state/auth_state.dart      ChangeNotifier: current user, login/register/logout, 401 → login
  util/                      permissions (role rules), validators (mirror API rules), formatting
  ui/widgets/                shared widgets, authenticated media (images, viewer, pickers), chart
  ui/screens/                one file per screen or screen group
```

- **State:** `provider` exposes the `ApiClient` and `AuthState`; screens load their own data
  (a small `Loader` widget handles loading / error / retry).
- **Auth:** the token lives in secure storage and is attached to every request. A
  `401 unauthorized` clears it and the router sends the user to the login screen
  (`401 invalid_credentials` on login/password change does not).
- **Roles:** actions a role can't perform (e.g. a `VIEWER` adding memories, a `MEMBER` editing
  someone else's memory) are hidden using `util/permissions.dart`; the server stays the
  authority and its `403/404/409` messages are shown when it refuses.
- **Media:** `content_url` needs the bearer token, so images use `Image.network` with the
  `Authorization` header; videos, audio and PDFs are downloaded to a temporary file and opened
  in the platform's default app. Uploads are multipart with parts named `file`.
- **Time capsules:** a capsule's message and media are only parsed and rendered when its
  status is `OPENED`; locked capsules show a live countdown.

### Screens

Login, Register, Dashboard (relationships grouped by profile), New profile + relationship,
Relationship (Timeline with search / date range / category / tag / order filters and infinite
scroll; Capsules; Members), Memory detail / create / edit (category, tags with suggestions,
gallery / camera / file attachments, gallery viewer), Profile (edit, relationships; Development
records by domain for children; Measurements with a chart per type), Time capsule detail /
create / edit (countdown, cancel, open, sealed attachments), Account (details, password,
sign out).

### Dependencies

| Package | Why |
|---------|-----|
| `http`, `http_parser` | REST + multipart client, `MockClient` for tests |
| `provider` | dependency injection of the client and auth state |
| `go_router` | routing with an auth redirect |
| `flutter_secure_storage` | bearer token in the Keystore / Keychain |
| `image_picker`, `file_picker` | photos/videos from gallery or camera; any file |
| `path_provider`, `open_filex` | temp files for downloads and opening them externally |

`path_provider_android` is pinned below 2.3 because 2.3 depends on `package:jni`, which needs an
NDK the read-only Nix SDK doesn't ship. For the same reason `android/build.gradle.kts` makes
plugin modules use the compileSdk / build-tools / NDK that `app/build.gradle.kts` selected.
The chart is a small `CustomPainter` rather than a charting package.
