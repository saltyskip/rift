import 'package:rift_flutter_ffi/src/rift_storage.dart';
import 'package:rift_flutter_ffi/src/rust/frb_generated.dart';

export 'package:rift_flutter_ffi/src/rust/frb_generated.dart'
    show ClickResult, GetLinkResult, DeferredDeepLinkResult;

const _kInstallId = 'rift.install_id';
const _kUserId = 'rift.user_id';
const _kUserIdSynced = 'rift.user_id_synced';

/// Rift SDK for Flutter. Obtain via [RiftSdk.create].
///
/// The SDK owns all attribution logic. The [RiftStorage] implementation you
/// provide owns persistence — the SDK reads initial state from storage at
/// construction and writes back after every state-mutating operation.
class RiftSdk {
  final RiftSdkRust _sdk;
  final RiftStorage _storage;

  RiftSdk._(this._sdk, this._storage);

  /// Initialize the SDK. Call once at app startup, before calling any other
  /// method. Loads persisted state (install_id, user binding) from [storage]
  /// and retries any unsynced user binding in the background.
  ///
  /// [storage] defaults to [SharedPrefsRiftStorage] when omitted.
  static Future<RiftSdk> create({
    required String publishableKey,
    RiftStorage? storage,
    String? baseUrl,
    String? logLevel,
    String? appVersion,
  }) async {
    await RustLib.init();

    final st = storage ?? await SharedPrefsRiftStorage.create();
    final installId = await st.get(_kInstallId);
    final userId = await st.get(_kUserId);
    final userIdSynced = await st.get(_kUserIdSynced) == 'true';

    final sdk = await RiftSdkRust.create(
      config: RiftConfig(
        publishableKey: publishableKey,
        baseUrl: baseUrl,
        logLevel: logLevel,
        appVersion: appVersion,
      ),
      state: installId != null
          ? RiftState(
              installId: installId,
              userId: userId,
              userIdSynced: userIdSynced,
            )
          : null,
    );

    // Persist a newly-generated install_id so it survives the next launch.
    final current = sdk.getState();
    if (current.installId != installId) {
      await st.set(_kInstallId, current.installId);
    }

    return RiftSdk._(sdk, st);
  }

  /// The persistent install ID for this device.
  String get installId => _sdk.getInstallId();

  /// Bind a user ID to this install. Call after the user signs in. Safe to
  /// call on every launch with the same user_id — it no-ops if already synced.
  Future<void> setUserId(String userId) async {
    final state = await _sdk.setUserId(userId: userId);
    await _persistState(state);
  }

  /// Clear the bound user ID. Call on logout. install_id is preserved.
  Future<void> clearUserId() async {
    final state = _sdk.clearUserId();
    await _persistState(state);
  }

  /// Resolve a link and return routing destinations.
  Future<ClickResult> click(String linkId) => _sdk.click(linkId: linkId);

  /// Report attribution for this install. Returns true on success.
  Future<bool> attributeLink(String linkId) => _sdk.attributeLink(linkId: linkId);

  /// Fetch link routing destinations without recording a click.
  Future<GetLinkResult> getLink(String linkId) => _sdk.getLink(linkId: linkId);

  /// Parse clipboard text for a Rift link, report attribution, and return link
  /// data for navigation. Returns null if no Rift link is found.
  ///
  /// Call this on your first stable screen (home/dashboard), NOT at cold start
  /// while the user is still in onboarding.
  Future<DeferredDeepLinkResult?> checkDeferredDeepLink({
    required String? clipboardText,
  }) =>
      _sdk.checkDeferredDeepLink(clipboardText: clipboardText);

  /// Fire a conversion event (purchase, signup, etc.). No-op if no user is bound.
  ///
  /// [metadata] is an optional JSON string of arbitrary key-value pairs.
  Future<void> trackConversion({
    required String type,
    required String idempotencyKey,
    String? metadata,
  }) =>
      _sdk.trackConversion(
        conversionType: type,
        idempotencyKey: idempotencyKey,
        metadata: metadata,
      );

  Future<void> _persistState(RiftState state) async {
    await _storage.set(_kInstallId, state.installId);
    if (state.userId != null) {
      await _storage.set(_kUserId, state.userId!);
    } else {
      await _storage.remove(_kUserId);
    }
    await _storage.set(_kUserIdSynced, state.userIdSynced ? 'true' : 'false');
  }
}
