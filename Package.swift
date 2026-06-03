// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "RiftSDK",
    platforms: [
        .iOS(.v15), .macOS(.v11)
    ],
    products: [
        .library(name: "RiftSDK", targets: ["RiftSDK"]),
    ],
    targets: [
        .binaryTarget(
            name: "rift_ffiFFI",
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.2.2/rift_ffiFFI.xcframework.zip",
            checksum: "e3cbf92223fa14f81b2e04a56c3315aff33b2eecfe956d3f18dd38edea34aedf"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "client/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
