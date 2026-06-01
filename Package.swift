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
            url: "https://github.com/saltyskip/rift/releases/download/sdk-v0.2.1/rift_ffiFFI.xcframework.zip",
            checksum: "d2594b3d529a21080242c46c0e25dafac93dfc75a4d062b660ac4dec5a3fef9d"
        ),
        .target(
            name: "RiftSDK",
            dependencies: ["rift_ffiFFI"],
            path: "client/mobile/dist/ios/Sources/RiftSDK"
        ),
    ]
)
