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
            name: "RiftSDKBinary",
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.1.0/RiftSDK.xcframework.zip",
            checksum: "PLACEHOLDER"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["RiftSDKBinary"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
