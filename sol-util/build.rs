use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
    
    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir).join("../../../");

    // Check if the IDL directory exists, if not create it
    let idl_dir = target_dir.join("idl");
    if !idl_dir.exists() {
        fs::create_dir_all(&idl_dir).expect("Failed to create IDL directory");
        println!("Created IDL directory at: {}", idl_dir.display());
    }

    // Print build information
    println!("Building sol-util with default configuration");

    // Notify about environment variable options
    println!("cargo:warning=You can set SOL_RPC_URL environment variable to change default RPC URL");
    
    // Check if we're building for release
    if env::var("PROFILE").unwrap_or_default() == "release" {
        println!("cargo:warning=Building in release mode");
        // Add any release-specific configurations here
    }

    // Additional build steps can go here
    println!("Build script completed successfully");
}
