use std::env;
use std::path::Path;

mod code_writer;
mod parser;

use code_writer::CodeWriter;
use parser::{CommandType, Parser};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <input.vm>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = get_output_filename(input_file);
    if let Err(e) = translate(input_file, &output_file) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    println!("Translation complete: {} -> {}", input_file, output_file);
}

fn translate(input_file: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(input_file)?;
    let mut code_writer = CodeWriter::new(output_file)?;

    // Set the filename for static variables
    code_writer.set_filename(input_file);

    while parser.has_more_commands() {
        parser.advance();

        match parser.command_type() {
            CommandType::Arithmetic => {
                let command = parser.arg1();
                code_writer.write_arithmetic(command)?;
            }
            CommandType::Push => {
                let segment = parser.arg1();
                let index = parser.arg2();
                code_writer.write_push_pop("push", segment, index)?;
            }
            CommandType::Pop => {
                let segment = parser.arg1();
                let index = parser.arg2();
                code_writer.write_push_pop("pop", segment, index)?;
            }
            _ => {
                // Other command types not implemented yet
                eprintln!(
                    "Warning: Command type not implemented: {:?}",
                    parser.command_type()
                );
            }
        }
    }

    code_writer.close()?;
    Ok(())
}

#[inline]
fn get_output_filename(input_file: &str) -> String {
    let path = Path::new(input_file);

    // More efficient path handling
    match (path.file_stem(), path.parent()) {
        (Some(stem), Some(parent)) => {
            let mut output = parent.as_os_str().to_string_lossy().into_owned();
            if !output.is_empty() {
                output.push('/');
            }
            output.push_str(&stem.to_string_lossy());
            output.push_str(".asm");
            output
        }
        (Some(stem), None) => {
            let mut output = stem.to_string_lossy().into_owned();
            output.push_str(".asm");
            output
        }
        _ => {
            // Fallback for edge cases
            format!("{}.asm", input_file)
        }
    }
}
