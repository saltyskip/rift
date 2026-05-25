import Flutter

// flutter_rust_bridge handles all FFI via dart:ffi.
// This class only exists to satisfy Flutter's plugin registration mechanism.
public class RiftFlutterFfiPlugin: NSObject, FlutterPlugin {
    public static func register(with registrar: FlutterPluginRegistrar) {}
}
