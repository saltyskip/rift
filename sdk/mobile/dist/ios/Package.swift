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
            path: "RiftSDK.xcframework"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["RiftSDKBinary"],
            path: "Sources/RiftSDK"
        ),
    ]
)
