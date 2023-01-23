use std::path::Path;
use std::path::PathBuf;

fn main() {
    // cargo build --target ./x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem

    let bin = "corgos-boot";
    let local_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!(
        "cargo:rustc-link-arg-bin={bin}=-T{}",
        local_path.join("boot.ld").display()
    );

    println!("cargo:rustc-link-arg-bin={bin}=-no-pie");
    println!("cargo:rustc-link-arg-bin={bin}=-e");
    println!("cargo:rustc-link-arg-bin={bin}=uefi_entry");
}
