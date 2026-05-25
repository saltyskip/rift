/// Rift Flutter SDK — deep link attribution, deferred deep linking, conversion tracking.
///
/// Quick start:
/// ```dart
/// final rift = await RiftSdk.create(publishableKey: 'pk_live_...');
///
/// // Bind user after sign-in
/// await rift.setUserId('user_123');
///
/// // Check for deferred deep link on first stable screen (not at launch)
/// final text = (await Clipboard.getData('text/plain'))?.text;
/// final result = await rift.checkDeferredDeepLink(clipboardText: text);
/// if (result != null) { /* navigate */ }
///
/// // Fire a conversion event
/// await rift.trackConversion(type: 'purchase', idempotencyKey: 'order_xyz');
/// ```
library rift_flutter_ffi;

export 'src/rift_sdk_impl.dart';
export 'src/rift_storage.dart';
