static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

use std::fs;
use std::path::Path;

fn main() {
    // Entry point to insert
    let entry_point = std::env::var("ENTRY_POINT").unwrap_or_else(|_| "kmain".to_string());

    // Load the template file
    let template = fs::read_to_string("src/entry.asm").expect("Failed to read entry.asm");

    // Replace placeholder
    let asm = template.replace("{entry_point}", entry_point.as_str());

    // Write the generated assembly to OUT_DIR
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("entry.S");
    fs::write(&dest_path, asm).expect("Failed to write entry.S");

    println!("cargo:rerun-if-changed=entry_template.S");

    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
}
