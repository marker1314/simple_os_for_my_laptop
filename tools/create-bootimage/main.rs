use std::process;

fn main() {
    eprintln!("This helper is deprecated. Use 'build-bootimage.ps1' to create a bootable image with bootloader 0.11.*.");
    eprintln!("Example: powershell -ExecutionPolicy Bypass -File build-bootimage.ps1");
    process::exit(2);
}

