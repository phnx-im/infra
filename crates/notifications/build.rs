fn main() {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    build_swift();
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn build_swift() {
    use swift_rs::SwiftLinker;

    const PACKAGE_NAME: &str = "Notifications";
    const PACKAGE_PATH: &str = "./swift";
    SwiftLinker::new("10.14")
        .with_ios("12")
        .with_package(PACKAGE_NAME, PACKAGE_PATH)
        .link();
}
