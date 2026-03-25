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
            checksum: "895aa5dd8b5f7646ec0e8fede23948fde949fd66593686916b77474a276c1255"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["RiftSDKBinary"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
