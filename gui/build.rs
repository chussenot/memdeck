fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let c_sources = [
        "../src/audio_engine.c",
        "../src/audio_mix.c",
        "../src/audio_seq.c",
        "../src/audio_dsp.c",
        "../src/audio_fx.c",
        "../src/audio_song_builtin.c",
        "../src/abc.c",
        "../src/card.c",
    ];

    for file in c_sources {
        println!("cargo:rerun-if-changed={file}");
    }
    println!("cargo:rerun-if-changed=../src/memdeck.h");
    println!("cargo:rerun-if-changed=../src/audio_engine.h");
    println!("cargo:rerun-if-changed=../src/audio_mix.h");
    println!("cargo:rerun-if-changed=../src/audio_seq.h");
    println!("cargo:rerun-if-changed=../src/audio_dsp.h");
    println!("cargo:rerun-if-changed=../src/audio_fx.h");
    println!("cargo:rerun-if-changed=../src/audio_song_builtin.h");
    println!("cargo:rerun-if-changed=../src/miniaudio.h");
    println!("cargo:rerun-if-changed=../src/miniaudio_playback.h");
    println!("cargo:rerun-if-changed=../src/miniaudio_playback.c");

    let mut build = cc::Build::new();
    build
        .include("../src")
        .define("_DEFAULT_SOURCE", None)
        .define("_XOPEN_SOURCE", Some("600"))
        .flag_if_supported("-std=c99")
        .warnings(true)
        .extra_warnings(true)
        .files(c_sources);

    build.compile("memdeck_audio_engine");

    // miniaudio playback backend — compiled separately so MINIAUDIO_IMPLEMENTATION
    // is defined in exactly one translation unit.
    cc::Build::new()
        .include("../src")
        .define("_DEFAULT_SOURCE", None)
        .flag_if_supported("-std=c99")
        // miniaudio is a large header; silence the warnings it triggers.
        .warnings(false)
        .file("../src/miniaudio_playback.c")
        .compile("memdeck_miniaudio");

    println!("cargo:rustc-link-lib=ncursesw");
    println!("cargo:rustc-link-lib=m");

    // Platform-specific libraries required by miniaudio.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-lib=framework=CoreAudio");
            println!("cargo:rustc-link-lib=framework=AudioToolbox");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=ole32");
            println!("cargo:rustc-link-lib=winmm");
        }
        _ => {
            // Linux and other Unix: miniaudio loads ALSA/PulseAudio at runtime
            // via dlopen — only dl and pthread are needed at link time.
            println!("cargo:rustc-link-lib=dl");
            println!("cargo:rustc-link-lib=pthread");
        }
    }
}
