import 'package:shared_preferences/shared_preferences.dart';

/// Storage backend contract. Implement this to use your preferred storage
/// (FlutterSecureStorage, Hive, encrypted SharedPreferences, etc.).
///
/// The default implementation, [SharedPrefsRiftStorage], uses
/// `shared_preferences` and is suitable for most apps. On Android this
/// survives app updates but not reinstalls. On iOS use [FlutterSecureStorage]
/// for Keychain-backed persistence across reinstalls.
abstract class RiftStorage {
  Future<String?> get(String key);
  Future<void> set(String key, String value);
  Future<void> remove(String key);
}

/// Default storage backed by `shared_preferences`.
class SharedPrefsRiftStorage implements RiftStorage {
  final SharedPreferences _prefs;

  const SharedPrefsRiftStorage(this._prefs);

  static Future<SharedPrefsRiftStorage> create() async {
    final prefs = await SharedPreferences.getInstance();
    return SharedPrefsRiftStorage(prefs);
  }

  @override
  Future<String?> get(String key) async => _prefs.getString(key);

  @override
  Future<void> set(String key, String value) async {
    await _prefs.setString(key, value);
  }

  @override
  Future<void> remove(String key) async {
    await _prefs.remove(key);
  }
}
