fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-static-pie");
    println!("cargo:rustc-link-arg=-fuse-ld=lld");
    println!("cargo:rustc-link-arg=-mcmodel=kernel");
    println!("cargo:rustc-link-arg=-T./build/kernel_start.lds");
}
