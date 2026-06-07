fn main() {
    println!("cargo:rerun-if-changed=src/darwin/native.c");
    println!("cargo:rerun-if-changed=src/darwin/native.h");

    cc::Build::new()
        .file("src/darwin/native.c")
        .warnings(true)
        .compile("macvmtop_native");
}
