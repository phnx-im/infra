// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "Notifications",
  platforms: [
    .iOS(.v12),
    .macOS(.v10_14),
  ],
  products: [
    .library(
      name: "Notifications",
      type: .static,
      targets: ["Notifications"]
    )
  ],
  targets: [
    .target(
      name: "Notifications"),
    .testTarget(
      name: "NotificationsTests",
      dependencies: ["Notifications"]
    ),
  ]
)
