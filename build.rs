// Build script for Simple OS
// Bootimage creation is handled separately via build-bootimage.ps1

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
}

