fn main() {
    println!("cargo:rerun-if-changed=src/macos_speech.swift");
    println!("cargo:rerun-if-changed=src/macos_speech_info.plist");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("Cargo sets OUT_DIR");
    let helper = std::path::Path::new(&out_dir).join("chinese-macos-speech");
    let status = std::process::Command::new("swiftc")
        .args([
            "-O",
            "-framework",
            "Speech",
            "-framework",
            "AVFoundation",
            "-Xlinker",
            "-sectcreate",
            "-Xlinker",
            "__TEXT",
            "-Xlinker",
            "__info_plist",
            "-Xlinker",
            "src/macos_speech_info.plist",
            "src/macos_speech.swift",
            "-o",
        ])
        .arg(&helper)
        .status()
        .expect("swiftc is required to build macOS microphone Speak support");
    if !status.success() {
        panic!("failed to build macOS microphone Speak helper");
    }
    println!("cargo:rustc-env=CHINESE_SPEECH_HELPER={}", helper.display());
}
