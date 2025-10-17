use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the project root directory
fn get_project_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    PathBuf::from(manifest_dir)
}

/// Find all .vm files in the test_data directory recursively
fn find_vm_files() -> Vec<PathBuf> {
    let mut vm_files = Vec::new();
    let project_root = get_project_root();
    let test_dir = project_root.join("test_data");

    if !test_dir.exists() {
        return vm_files;
    }

    visit_dirs(&test_dir, &mut vm_files).ok();
    vm_files.sort();
    vm_files
}

/// Recursively visit directories to find .vm files
fn visit_dirs(dir: &Path, vm_files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, vm_files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("vm") {
                vm_files.push(path);
            }
        }
    }
    Ok(())
}

/// Run the translator on a .vm file to a temporary output file
fn run_translator_to_temp(vm_file: &Path) -> Result<PathBuf, String> {
    let project_root = get_project_root();

    // Create a temporary output file path
    let temp_asm = vm_file.with_extension("temp.asm");

    // First, create a temporary copy of the vm file to avoid modifying the original
    let temp_vm = vm_file.with_extension("temp.vm");
    fs::copy(vm_file, &temp_vm).map_err(|e| format!("Failed to create temp vm file: {}", e))?;

    let output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--quiet")
        .arg("--")
        .arg(&temp_vm)
        .current_dir(&project_root)
        .output()
        .map_err(|e| format!("Failed to run translator: {}", e))?;

    // Clean up temp vm file
    fs::remove_file(&temp_vm).ok();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Translator failed: {}", stderr));
    }

    // The translator generates output with the same base name as input
    let generated_asm = temp_vm.with_extension("asm");

    if !generated_asm.exists() {
        return Err(format!("Output file not created: {:?}", generated_asm));
    }

    // Rename to our temp file
    fs::rename(&generated_asm, &temp_asm)
        .map_err(|e| format!("Failed to rename output file: {}", e))?;

    Ok(temp_asm)
}

/// Compare two files line by line
fn compare_files(actual: &Path, expected: &Path) -> Result<(), String> {
    let actual_content =
        fs::read_to_string(actual).map_err(|e| format!("Failed to read actual file: {}", e))?;
    let expected_content =
        fs::read_to_string(expected).map_err(|e| format!("Failed to read expected file: {}", e))?;

    if actual_content == expected_content {
        return Ok(());
    }

    // Find the first difference for better error messages
    let actual_lines: Vec<&str> = actual_content.lines().collect();
    let expected_lines: Vec<&str> = expected_content.lines().collect();

    let mut diff_lines = Vec::new();
    let max_len = actual_lines.len().max(expected_lines.len());

    for i in 0..max_len.min(10) {
        // Show first 10 differences
        let actual_line = actual_lines.get(i).unwrap_or(&"<EOF>");
        let expected_line = expected_lines.get(i).unwrap_or(&"<EOF>");

        if actual_line != expected_line {
            diff_lines.push(format!(
                "Line {}: \n  Expected: {}\n  Actual:   {}",
                i + 1,
                expected_line,
                actual_line
            ));
        }
    }

    if diff_lines.is_empty() {
        Err(format!(
            "Files differ in length: actual {} lines, expected {} lines",
            actual_lines.len(),
            expected_lines.len()
        ))
    } else {
        Err(format!("Files differ:\n{}", diff_lines.join("\n")))
    }
}

/// Clean up all temporary test files and generated .asm files (except .expected.asm)
fn cleanup_temp_files() {
    let vm_files = find_vm_files();
    for vm_file in vm_files {
        // Remove temp.asm files
        let temp_asm = vm_file.with_extension("temp.asm");
        if temp_asm.exists() {
            fs::remove_file(&temp_asm).ok();
        }

        // Remove temp.vm files
        let temp_vm = vm_file.with_extension("temp.vm");
        if temp_vm.exists() {
            fs::remove_file(&temp_vm).ok();
        }

        // Remove generated .asm files (but keep .expected.asm)
        let asm_file = vm_file.with_extension("asm");
        let expected_file = vm_file.with_extension("expected.asm");
        if asm_file.exists() && asm_file != expected_file {
            fs::remove_file(&asm_file).ok();
        }
    }
}

/// Generate expected output files for all tests
#[test]
#[ignore] // Use `cargo test -- --ignored` to run this
fn generate_expected_files() {
    let vm_files = find_vm_files();
    assert!(!vm_files.is_empty(), "No .vm test files found!");

    let mut generated = 0;

    for vm_file in vm_files {
        println!("Generating expected output for: {:?}", vm_file);

        match run_translator_to_temp(&vm_file) {
            Ok(temp_asm) => {
                let expected_file = vm_file.with_extension("expected.asm");
                fs::copy(&temp_asm, &expected_file)
                    .unwrap_or_else(|e| panic!("Failed to copy to expected file: {}", e));

                // Clean up temp file
                fs::remove_file(&temp_asm).ok();

                println!("  ✓ Generated: {:?}", expected_file);
                generated += 1;
            }
            Err(e) => {
                println!("  ⚠ Warning: {}", e);
            }
        }
    }

    // Final cleanup
    cleanup_temp_files();

    println!("\nGenerated {} expected output files", generated);
}

/// Test all .vm files by comparing output with expected
#[test]
fn test_all_vm_files() {
    let vm_files = find_vm_files();
    assert!(
        !vm_files.is_empty(),
        "No .vm test files found in test_data/ directory!"
    );

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    println!("\n========================================");
    println!("Running VM Translator Tests");
    println!("========================================\n");

    for vm_file in &vm_files {
        let test_name = format!(
            "{}/{}",
            vm_file
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy(),
            vm_file.file_stem().unwrap().to_string_lossy()
        );

        print!("Testing: {} ... ", test_name);

        let expected_file = vm_file.with_extension("expected.asm");

        // Check if expected file exists
        if !expected_file.exists() {
            println!("⚠ SKIPPED (no expected file)");
            skipped += 1;
            continue;
        }

        // Run translator to temp file
        let temp_asm = match run_translator_to_temp(vm_file) {
            Ok(file) => file,
            Err(e) => {
                println!("✗ FAILED (translator error)");
                failures.push(format!("{}: {}", test_name, e));
                failed += 1;
                continue;
            }
        };

        // Compare output
        match compare_files(&temp_asm, &expected_file) {
            Ok(_) => {
                println!("✓ PASSED");
                passed += 1;
            }
            Err(e) => {
                println!("✗ FAILED");
                failures.push(format!("{}: {}", test_name, e));
                failed += 1;
            }
        }

        // Clean up temp file immediately after test
        fs::remove_file(&temp_asm).ok();
    }

    // Final cleanup to ensure all temp files are removed
    cleanup_temp_files();

    // Print summary
    println!("\n========================================");
    println!("Test Summary");
    println!("========================================");
    println!("Passed:  {}", passed);
    println!("Failed:  {}", failed);
    println!("Skipped: {}", skipped);
    println!("Total:   {}", vm_files.len());
    println!("========================================\n");

    // Print failures if any
    if !failures.is_empty() {
        println!("Failed tests:");
        for failure in &failures {
            println!("\n{}", failure);
        }
    }

    // Assert all tests passed
    if failed > 0 {
        panic!("{} test(s) failed out of {}", failed, vm_files.len());
    }
}

/// Cleanup test to remove any lingering temp files and generated .asm files
/// Run this with: cargo test cleanup_all_test_files -- --ignored
#[test]
#[ignore]
fn cleanup_all_test_files() {
    cleanup_temp_files();
    println!("Cleaned up all temporary test files");
}
