//! Hack Assembler - Main Entry Point
//!
//! A two-pass assembler for the Hack assembly language (`Nand2Tetris` Project 6).
//!
//! # Architecture
//! - **Pass 1**: Builds the symbol table by recording label positions
//! - **Pass 2**: Generates machine code, resolving all symbols
//!
//! # Usage
//! ```bash
//! cargo run <input.asm> [output.hack]
//! ```

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process;

mod code;
mod parser;
mod symbol_table;

use parser::{CommandType, ParserLines};
use symbol_table::SymbolTable;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Reads assembly file into memory
fn read_lines(path: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    reader
        .lines()
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// First pass: Build symbol table with label addresses
///
/// Scans through all lines and records the ROM address of each label.
/// Label definitions (L-commands) don't generate code, so they don't
/// increment the ROM address counter.
fn first_pass(lines: &[String], symbol_table: &mut SymbolTable) -> Result<()> {
    let mut rom_address = 0u16;
    let mut parser = ParserLines::from_lines(lines);

    while parser.advance() {
        match parser.command_type()? {
            CommandType::LCommand => {
                // Labels mark the next instruction's address
                let symbol = parser.symbol()?;
                symbol_table.add_entry(symbol, rom_address);
            }
            CommandType::ACommand | CommandType::CCommand => {
                // Actual instructions increment the address
                rom_address += 1;
            }
        }
    }

    Ok(())
}

/// Second pass: Generate machine code
///
/// Translates each instruction to binary:
/// - A-commands: Resolve symbols to addresses
/// - C-commands: Encode dest, comp, and jump fields
/// - L-commands: Skip (already processed in pass 1)
fn second_pass(
    lines: &[String],
    symbol_table: &mut SymbolTable,
    writer: &mut BufWriter<File>,
) -> Result<()> {
    let mut ram_address = 16u16; // Variables start at RAM[16]
    let mut parser = ParserLines::from_lines(lines);

    while parser.advance() {
        match parser.command_type()? {
            CommandType::ACommand => {
                let symbol = parser.symbol()?;

                // Try to parse as number first, then lookup/insert as symbol
                let address = symbol
                    .parse::<u16>()
                    .unwrap_or_else(|_| symbol_table.get_or_insert(symbol, &mut ram_address));

                let instruction = code::encode_a_instruction(address);
                writeln!(writer, "{instruction}")?;
            }
            CommandType::CCommand => {
                let dest = parser.dest()?.unwrap_or("");
                let comp = parser.comp()?.unwrap_or("");
                let jump = parser.jump()?.unwrap_or("");

                let instruction = code::encode_c_instruction(dest, comp, jump);
                writeln!(writer, "{instruction}")?;
            }
            CommandType::LCommand => {
                panic!("L command not supported");
            }
        }
    }

    writer.flush()?;
    Ok(())
}

/// Determines the output file path
fn output_path(input: &str, explicit_output: Option<&str>) -> String {
    explicit_output.map_or_else(
        || input.replace(".asm", ".hack"),
        std::string::ToString::to_string,
    )
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Validate arguments
    if !(2..=3).contains(&args.len()) {
        eprintln!("Usage: {} <input.asm> [output.hack]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} Add.asm", args[0]);
        eprintln!("  {} Add.asm Add.hack", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];

    // Read source file
    let lines = read_lines(input_path)?;

    // Initialize symbol table with predefined symbols
    let mut symbol_table = SymbolTable::new();

    // Pass 1: Build symbol table
    first_pass(&lines, &mut symbol_table)?;

    // Pass 2: Generate machine code
    let output = output_path(input_path, args.get(2).map(String::as_str));
    let output_file = File::create(&output)?;
    let mut writer = BufWriter::new(output_file);

    second_pass(&lines, &mut symbol_table, &mut writer)?;

    println!("Assembly completed. Output written to {output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_path_generation() {
        assert_eq!(output_path("test.asm", None), "test.hack");
        assert_eq!(output_path("test.asm", Some("custom.hack")), "custom.hack");
        assert_eq!(output_path("dir/file.asm", None), "dir/file.hack");
        assert_eq!(output_path("path/to/file.asm", None), "path/to/file.hack");
    }

    #[test]
    fn test_output_path_explicit() {
        assert_eq!(output_path("any.asm", Some("out.hack")), "out.hack");
        assert_eq!(
            output_path("any.asm", Some("path/to/out.hack")),
            "path/to/out.hack"
        );
    }
}
