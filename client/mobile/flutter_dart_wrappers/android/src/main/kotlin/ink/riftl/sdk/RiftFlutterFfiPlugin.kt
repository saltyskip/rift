package ink.riftl.sdk

import io.flutter.embedding.engine.plugins.FlutterPlugin

// flutter_rust_bridge handles all FFI via dart:ffi.
// This class only exists to satisfy Flutter's plugin registration mechanism.
class RiftFlutterFfiPlugin : FlutterPlugin {
    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {}
    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {}
}
