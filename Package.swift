// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "RiftSDK",
    platforms: [
        .iOS(.v14), .macOS(.v11)
    ],
    products: [
        .library(name: "RiftSDK", targets: ["RiftSDK"]),
    ],
    targets: [
        .binaryTarget(
            name: "rift_ffiFFI",
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.1.0/RiftSDK.xcframework.zip",
            checksum: "8eee0bdf417b66bd112edd995b7ec94119579c905e1d9dd7d0d5d0fbf8014cb7"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
