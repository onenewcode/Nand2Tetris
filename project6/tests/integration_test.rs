use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_all_asm_files() {
    // Scan the tests directory for .asm files
    let tests_dir = Path::new("tests");

    // Walk through all subdirectories
    for entry in fs::read_dir(tests_dir).expect("Cannot read tests directory") {
        let entry = entry.expect("Cannot read directory entry");
        let path = entry.path();

        if path.is_dir() {
            test_asm_files_in_directory(&path);
        }
    }
}

fn test_asm_files_in_directory(dir: &Path) {
    // Collect all files in the directory
    let entries: Vec<_> = fs::read_dir(dir)
        .expect("Cannot read directory")
        .map(|e| e.expect("Cannot read directory entry"))
        .collect();

    // Separate .asm files and .hack files
    let asm_files: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "asm"))
        .map(|e| e.path())
        .collect();

    let hack_files: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "hack"))
        .map(|e| e.path())
        .collect();

    // Test each .asm file
    for asm_file in &asm_files {
        test_single_asm_file(asm_file, &hack_files);
    }
}

fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace("\r", "\n")
}

fn test_single_asm_file(input_path: &Path, reference_files: &[std::path::PathBuf]) {
    println!("Testing file: {}", input_path.display());

    // Generate temporary output path
    let temp_output_path = input_path.with_extension("temp.hack");

    // Find the corresponding reference file
    let reference_path = find_reference_file(input_path, reference_files);

    // Run the assembler with specified output path
    let status = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--")
        .arg(input_path.to_str().unwrap())
        .arg(temp_output_path.to_str().unwrap())
        .status()
        .expect("Failed to execute assembler");

    // Check that the assembler ran successfully
    assert!(
        status.success(),
        "Assembler failed for {}",
        input_path.display()
    );

    // Compare the output with reference if reference exists
    if let Some(ref_path) = reference_path {
        let generated = fs::read_to_string(&temp_output_path).unwrap_or_else(|_| panic!("Cannot read generated file: {}",
            temp_output_path.display()));
        let reference = fs::read_to_string(&ref_path).unwrap_or_else(|_| panic!("Cannot read reference file: {}",
            ref_path.display()));

        // Normalize line endings before comparison
        let generated_normalized = normalize_line_endings(&generated);
        let reference_normalized = normalize_line_endings(&reference);

        assert_eq!(
            generated_normalized,
            reference_normalized,
            "Generated code does not match reference for {}",
            input_path.display()
        );

        // Clean up temporary file
        fs::remove_file(&temp_output_path).expect("Failed to remove temporary file");
    } else {
        panic!("Invalid test case: {}", input_path.display());
    }
}

fn find_reference_file(
    input_path: &Path,
    reference_files: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    let input_filename = input_path.file_name().unwrap().to_str().unwrap();

    // Try to find an exact match first
    for ref_file in reference_files {
        let ref_filename = ref_file.file_name().unwrap().to_str().unwrap();
        let ref_stem = ref_file.file_stem().unwrap().to_str().unwrap();

        // Direct match
        if ref_filename == input_filename {
            return Some(ref_file.clone());
        }

        // Match without 'L' suffix (e.g., Max.asm -> MaxL.hack)
        if input_filename.strip_suffix(".asm")
            == Some(ref_stem.strip_suffix('L').unwrap_or(ref_stem))
        {
            return Some(ref_file.clone());
        }
    }

    // If no specific reference found, use the default output file as reference if it exists
    let default_ref = input_path.with_extension("hack");
    if default_ref.exists() && reference_files.iter().any(|f| f == &default_ref) {
        Some(default_ref)
    } else {
        None
    }
}
