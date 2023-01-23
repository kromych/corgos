use std::path::Path;

fn main() {
    let local_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:rustc-link-arg-bin=--script={}",
        local_path.join("boot.ld").display()
    );

    println!("cargo:rustc-link-arg-bin=-no-pie");
}
