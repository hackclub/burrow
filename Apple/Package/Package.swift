// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Burrow",
    platforms: [
        .macOS(.v14),
        .iOS(.v17)
    ],
    products: [
        .library(name: "BurrowUI", targets: ["BurrowUI"]),
        .library(name: "BurrowCore", targets: ["BurrowCore"]),
        .library(name: "BurrowClient", targets: ["BurrowClient"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.27.0"),
        .package(url: "https://github.com/grpc/grpc-swift.git", from: "1.23.0"),
    ],
    targets: [
        .target(name: "BurrowUI", dependencies: [
            .target(name: "BurrowCore"),
            .product(name: "SwiftProtobuf", package: "swift-protobuf"),
        ]),
        .target(name: "BurrowCore", dependencies: [.target(name: "BurrowClient")]),
        .target(
            name: "BurrowClient",
            dependencies: [
                .product(name: "SwiftProtobuf", package: "swift-protobuf"),
                .product(name: "GRPC", package: "grpc-swift"),
            ],
            plugins: [
                .plugin(name: "SwiftProtobufPlugin", package: "swift-protobuf"),
                .plugin(name: "GRPCSwiftPlugin", package: "grpc-swift"),
            ]
        ),
    ]
)
