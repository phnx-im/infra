pub fn main() {
    #[cfg(feature = "dart-bridge")]
    {
        std::process::Command::new("make")
            .current_dir("dart-bridge")
            .arg("dart-bridge")
            .output()
            .unwrap();
    }
}
