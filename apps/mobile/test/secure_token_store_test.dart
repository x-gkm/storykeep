import 'package:flutter/services.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:storykeep/api/secure_token_store.dart';

/// A keystore that fails like `flutter_secure_storage` does on a Linux desktop
/// without a Secret Service provider.
class _UnavailableStorage implements FlutterSecureStorage {
  int calls = 0;

  @override
  dynamic noSuchMethod(Invocation invocation) {
    calls++;
    return Future<Never>.error(
      PlatformException(
        code: 'Libsecret error',
        message: 'secret_service_get_sync: The name is not activatable',
      ),
    );
  }
}

void main() {
  test('falls back to memory when the keystore is unavailable', () async {
    final storage = _UnavailableStorage();
    final store = SecureTokenStore(storage);

    expect(await store.read(), isNull);
    expect(store.usingFallback, isTrue);

    await store.write('abc123');
    expect(await store.read(), 'abc123');
    await store.delete();
    expect(await store.read(), isNull);

    // Once the keystore has failed it isn't retried on every call.
    expect(storage.calls, 1);
  });
}
