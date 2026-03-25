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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.2.0/rift_ffiFFI.xcframework.zip",
            checksum: "5cafefb6b9a3fbc88484b4df0cafdf4cab6bf35b4bafb3579541696440896de3"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "sdk/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
