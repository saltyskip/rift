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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.0.1/rift_ffiFFI.xcframework.zip",
            checksum: "17d692210c6226bb010eb601e7db68ec0fcb0a1a8639f60392d20331f9e7536c"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
