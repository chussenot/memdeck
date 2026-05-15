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

    println!("cargo:rustc-link-lib=ncursesw");
    println!("cargo:rustc-link-lib=m");
}
