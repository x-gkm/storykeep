import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'token_store.dart';

/// Stores the token in the platform keystore (Android Keystore / iOS Keychain /
/// Secret Service on Linux).
///
/// If the keystore is unavailable — e.g. a Linux desktop without a running
/// keyring (GNOME Keyring, KWallet) — the token is kept in memory instead: the
/// user stays signed in until the app quits rather than the token being written
/// to disk unencrypted.
class SecureTokenStore implements TokenStore {
  SecureTokenStore([FlutterSecureStorage? storage])
    : _storage = storage ?? const FlutterSecureStorage();

  static const _key = 'storykeep.token';
  final FlutterSecureStorage _storage;
  MemoryTokenStore? _fallback;

  /// Whether the keystore failed and tokens now only live in memory.
  bool get usingFallback => _fallback != null;

  Future<T> _guard<T>(
    Future<T> Function(FlutterSecureStorage storage) secure,
    Future<T> Function(MemoryTokenStore memory) memory,
  ) async {
    final fallback = _fallback;
    if (fallback != null) return memory(fallback);
    try {
      return await secure(_storage);
    } on PlatformException catch (error) {
      debugPrint(
        'Secure storage unavailable, keeping the token in memory: ${error.message}',
      );
      return memory(_fallback = MemoryTokenStore());
    }
  }

  @override
  Future<String?> read() => _guard((s) => s.read(key: _key), (m) => m.read());

  @override
  Future<void> write(String token) =>
      _guard((s) => s.write(key: _key, value: token), (m) => m.write(token));

  @override
  Future<void> delete() =>
      _guard((s) => s.delete(key: _key), (m) => m.delete());
}
