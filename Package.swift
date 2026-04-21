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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.1.3/rift_ffiFFI.xcframework.zip",
            checksum: "4bbcb9fe20487cb60ce2c6116d27d32943e50ce1fa3881a0e69063a330dacf38"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "client/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
