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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.0.2/rift_ffiFFI.xcframework.zip",
            checksum: "c3f98917f4825af56217cfa85c9175deb2319b5330b1a360e1c323a87a9f4531"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
