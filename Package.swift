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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.1.1/rift_ffiFFI.xcframework.zip",
            checksum: "d97cbe5c820e0d520b2aab68869ed548ac11ef6f4169efb1766cc2d61c07c89f"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "client/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
