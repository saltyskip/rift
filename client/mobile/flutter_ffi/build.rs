fn main() {
    // flutter_rust_bridge's proc macros emit `cfg(frb_expand)` internally.
    // Declaring it here avoids the "unexpected cfg condition" warning when
    // building without having run `flutter_rust_bridge_codegen generate` first.
    println!("cargo::rustc-check-cfg=cfg(frb_expand)");
}
