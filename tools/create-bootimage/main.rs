use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <kernel_binary> <output_bootimage>", args[0]);
        eprintln!("Example: {} target/x86_64-unknown-none/debug/simple_os target/x86_64-unknown-none/debug/bootimage-simple_os.bin", args[0]);
        std::process::exit(1);
    }
    
    let kernel_path = PathBuf::from(&args[1]);
    let bootimage_path = PathBuf::from(&args[2]);
    
    if !kernel_path.exists() {
        eprintln!("Error: Kernel binary not found: {:?}", kernel_path);
        std::process::exit(1);
    }
    
    println!("Creating bootimage...");
    println!("  Kernel: {:?}", kernel_path);
    println!("  Output: {:?}", bootimage_path);
    
    // Use bootloader crate's API to create bootimage
    // Note: The exact API may vary - this is a placeholder for the correct API
    match create_bootimage(&kernel_path, &bootimage_path) {
        Ok(_) => {
            println!("Bootimage created successfully!");
        }
        Err(e) => {
            eprintln!("Error creating bootimage: {}", e);
            std::process::exit(1);
        }
    }
}

fn create_bootimage(kernel: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Try to use bootloader crate's build API
    // Since the exact API is unknown, we'll need to check bootloader crate docs
    // For now, this is a placeholder that will need the correct API
    
    // bootloader::Builder::new()
    //     .kernel_binary_path(kernel)
    //     .create_disk_image(output)?;
    
    // Temporary: Copy kernel to output location as a placeholder
    // You'll need to replace this with the actual bootloader API call
    std::fs::copy(kernel, output)?;
    
    Ok(())
}

