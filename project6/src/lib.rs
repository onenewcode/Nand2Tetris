//! Hack Assembler for the `Nand2Tetris` course
//!
//! This crate provides a high-performance Hack assembly language assembler that translates
//! assembly code into Hack machine code.
//!
//! # Architecture
//!
//! The assembler consists of four main modules:
//! - [`parser`]: Zero-copy parsing of assembly instructions
//! - [`code`]: Binary encoding using perfect hash functions (PHF)
//! - [`symbol_table`]: Symbol management with predefined symbols
//! - [`macros`]: Compile-time optimizations and utilities
//!
//! # Performance Optimizations
//!
//! - **PHF (Perfect Hash Functions)**: O(1) compile-time hash maps for instruction encoding
//! - **Zero-copy parsing**: Uses string slices to avoid allocations
//! - **Aggressive inlining**: Hot path functions are force-inlined
//! - **Pre-allocated capacity**: Reduces rehashing overhead
//! - **Link-time optimization (LTO)**: Enabled in release profile
//!
//! # Example
//!
//! ```rust
//! use project6::{ParserLines, CommandType, SymbolTable, code};
//!
//! // Parse assembly code
//! let lines = vec!["@100".to_string(), "D=M".to_string()];
//! let mut parser = ParserLines::from_lines(&lines);
//!
//! // Process first instruction
//! parser.advance();
//! assert_eq!(parser.command_type().unwrap(), CommandType::ACommand);
//! assert_eq!(parser.symbol().unwrap(), "100");
//!
//! // Process second instruction
//! parser.advance();
//! assert_eq!(parser.command_type().unwrap(), CommandType::CCommand);
//! let instruction = code::encode_c_instruction("D", "M", "");
//! assert_eq!(instruction, "1111110000010000");
//!
//! // Use symbol table
//! let mut symbols = SymbolTable::new();
//! symbols.add_entry("LOOP", 10);
//! assert_eq!(symbols.get_address("LOOP"), 10);
//! assert_eq!(symbols.get_address("SP"), 0); // Predefined symbol
//! ```

#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::inline_always,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

#[macro_use]
pub mod macros;

pub mod code;
pub mod parser;
pub mod symbol_table;

// Re-export commonly used types for convenience
pub use parser::{CommandType, ParserError, ParserLines};
pub use symbol_table::SymbolTable;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_assembly_workflow() {
        let lines = vec![
            "@2".to_string(),
            "D=A".to_string(),
            "@3".to_string(),
            "D=D+A".to_string(),
            "@0".to_string(),
            "M=D".to_string(),
        ];

        let mut parser = ParserLines::from_lines(&lines);
        let mut instructions = Vec::new();

        while parser.advance() {
            match parser.command_type().unwrap() {
                CommandType::ACommand => {
                    let symbol = parser.symbol().unwrap();
                    let addr = symbol.parse::<u16>().unwrap();
                    instructions.push(code::encode_a_instruction(addr));
                }
                CommandType::CCommand => {
                    let instruction = code::encode_c_instruction(
                        parser.dest().unwrap().unwrap_or(""),
                        parser.comp().unwrap().unwrap_or(""),
                        parser.jump().unwrap().unwrap_or(""),
                    );
                    instructions.push(instruction);
                }
                CommandType::LCommand => {}
            }
        }

        assert_eq!(instructions.len(), 6);
        assert_eq!(instructions[0], "0000000000000010"); // @2
        assert_eq!(instructions[1], "1110110000010000"); // D=A
    }

    #[test]
    fn test_symbol_table_integration() {
        let mut st = SymbolTable::new();
        let mut next_addr = 16;

        // Test predefined symbols
        assert_eq!(st.get_address("SP"), 0);
        assert_eq!(st.get_address("R15"), 15);
        assert_eq!(st.get_address("SCREEN"), 16384);

        // Test get_or_insert
        let var1 = st.get_or_insert("i", &mut next_addr);
        assert_eq!(var1, 16);
        assert_eq!(next_addr, 17);

        let var1_again = st.get_or_insert("i", &mut next_addr);
        assert_eq!(var1_again, 16);
        assert_eq!(next_addr, 17); // Should not increment
    }
}
