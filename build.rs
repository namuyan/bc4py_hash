extern crate cc;

/// build by GNU or MSVC
fn main() {
    if cfg!(feature = "hashs") {
        yespower_build();
        x16s_build();
        x11_build();
    } else {
        eprintln!("skip compile");
    }
}

/// yespower crypto hash library build
fn yespower_build() {
    let mut compiler = cc::Build::new();
    if !cfg!(windows) {
        // for only GNU
        compiler
            .flag("-march=native")
            .flag("-funroll-loops")
            .flag("-fomit-frame-pointer");
    }
    compiler
        .file("src/yespower/yespower.c")
        .include("src/yespower")
        .compile("yespower");
}

/// X16S crypto hash library build
fn x16s_build() {
    let files = [
        "src/x16s/x16s.c",
        "src/x16s/sha3/blake.c",
        "src/x16s/sha3/bmw.c",
        "src/x16s/sha3/groestl.c",
        "src/x16s/sha3/jh.c",
        "src/x16s/sha3/keccak.c",
        "src/x16s/sha3/skein.c",
        "src/x16s/sha3/cubehash.c",
        "src/x16s/sha3/echo.c",
        "src/x16s/sha3/luffa.c",
        "src/x16s/sha3/simd.c",
        "src/x16s/sha3/hamsi.c",
        "src/x16s/sha3/hamsi_helper.c",
        "src/x16s/sha3/fugue.c",
        "src/x16s/sha3/shavite.c",
        "src/x16s/sha3/shabal.c",
        "src/x16s/sha3/whirlpool.c",
        "src/x16s/sha3/sha2big.c",
    ];
    cc::Build::new()
        .files(&files)
        .include("src/x16s/sha3")
        .compile("x16s");
}

/// X11 crypto hash library build
fn x11_build() {
    let files = [
        "src/x11/x11hash.c",
        "src/x11/sha3/blake.c",
        "src/x11/sha3/bmw.c",
        "src/x11/sha3/groestl.c",
        "src/x11/sha3/jh.c",
        "src/x11/sha3/keccak.c",
        "src/x11/sha3/skein.c",
        "src/x11/sha3/cubehash.c",
        "src/x11/sha3/echo.c",
        "src/x11/sha3/luffa.c",
        "src/x11/sha3/simd.c",
        "src/x11/sha3/shavite.c",
    ];
    cc::Build::new()
        .files(&files)
        .include("src/x11/sha3")
        .compile("x11");
}
