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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.2.3/rift_ffiFFI.xcframework.zip",
            checksum: "ef197e17385140c78087e44a52ab52d3ca257975cc99c991cbbb6f35e01e03f1"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "client/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
