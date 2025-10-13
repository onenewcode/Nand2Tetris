//! Code generation module for Hack assembly language
//!
//! Translates assembly mnemonics to binary machine code using perfect hash functions (PHF).
//! PHF provides O(1) lookup with zero runtime overhead - the hash table is computed at compile time.
//!
//! # Performance
//! - All lookups use PHF maps: O(1) compile-time perfect hashing
//! - String formatting uses standard library (optimized by LLVM)
//! - Hot paths are inlined for better performance

use phf::phf_map;

/// Destination mnemonic to binary code mapping (3 bits)
///
/// Maps destination mnemonics to their 3-bit binary representation.
/// Empty string represents null destination.
static DEST_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "" => "000",
    "M" => "001",
    "D" => "010",
    "MD" => "011",
    "A" => "100",
    "AM" => "101",
    "AD" => "110",
    "AMD" => "111",
};

/// Computation mnemonic to binary code mapping (7 bits: 1 bit 'a' + 6 bits 'cccccc')
///
/// Includes both a=0 (A register) and a=1 (M register) variants.
/// The first bit indicates which register to use: 0 for A, 1 for M.
static COMP_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    // a=0 (A register operations)
    "0" => "0101010",
    "1" => "0111111",
    "-1" => "0111010",
    "D" => "0001100",
    "A" => "0110000",
    "!D" => "0001101",
    "!A" => "0110001",
    "-D" => "0001111",
    "-A" => "0110011",
    "D+1" => "0011111",
    "A+1" => "0110111",
    "D-1" => "0001110",
    "A-1" => "0110010",
    "D+A" => "0000010",
    "D-A" => "0010011",
    "A-D" => "0000111",
    "D&A" => "0000000",
    "D|A" => "0010101",

    // a=1 (M register operations)
    "M" => "1110000",
    "!M" => "1110001",
    "-M" => "1110011",
    "M+1" => "1110111",
    "M-1" => "1110010",
    "D+M" => "1000010",
    "D-M" => "1010011",
    "M-D" => "1000111",
    "D&M" => "1000000",
    "D|M" => "1010101",
};

/// Jump mnemonic to binary code mapping (3 bits)
///
/// Maps jump mnemonics to their 3-bit binary representation.
/// Empty string represents no jump.
static JUMP_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "" => "000",
    "JGT" => "001",
    "JEQ" => "010",
    "JGE" => "011",
    "JLT" => "100",
    "JNE" => "101",
    "JLE" => "110",
    "JMP" => "111",
};

/// Default values for missing mnemonics
const DEFAULT_DEST: &str = "000";
const DEFAULT_COMP: &str = "0101010"; // Computes 0
const DEFAULT_JUMP: &str = "000";

/// Translates a destination mnemonic to its binary code
///
/// # Arguments
/// * `mnemonic` - Destination mnemonic (e.g., "D", "M", "AMD")
///
/// # Returns
/// 3-bit binary string, or "000" if mnemonic is invalid
///
/// # Performance
/// Uses PHF for O(1) lookup with zero runtime overhead
#[inline]
#[must_use]
pub fn dest(mnemonic: &str) -> &'static str {
    DEST_MAP.get(mnemonic).copied().unwrap_or(DEFAULT_DEST)
}

/// Translates a computation mnemonic to its binary code
///
/// # Arguments
/// * `mnemonic` - Computation mnemonic (e.g., "D+1", "D&M")
///
/// # Returns
/// 7-bit binary string, or "0101010" (computes 0) if mnemonic is invalid
///
/// # Performance
/// Uses PHF for O(1) lookup with zero runtime overhead
#[inline]
#[must_use]
pub fn comp(mnemonic: &str) -> &'static str {
    COMP_MAP.get(mnemonic).copied().unwrap_or(DEFAULT_COMP)
}

/// Translates a jump mnemonic to its binary code
///
/// # Arguments
/// * `mnemonic` - Jump mnemonic (e.g., "JMP", "JEQ")
///
/// # Returns
/// 3-bit binary string, or "000" (no jump) if mnemonic is invalid
///
/// # Performance
/// Uses PHF for O(1) lookup with zero runtime overhead
#[inline]
#[must_use]
pub fn jump(mnemonic: &str) -> &'static str {
    JUMP_MAP.get(mnemonic).copied().unwrap_or(DEFAULT_JUMP)
}

/// Encodes a complete C-instruction
///
/// C-instruction format: 111accccccdddjjj (16 bits)
/// - 111: C-instruction prefix (3 bits)
/// - acccccc: computation (7 bits)
/// - ddd: destination (3 bits)
/// - jjj: jump (3 bits)
///
/// # Arguments
/// * `dest_mnemonic` - Destination mnemonic
/// * `comp_mnemonic` - Computation mnemonic
/// * `jump_mnemonic` - Jump mnemonic
///
/// # Returns
/// 16-bit binary string
///
/// # Example
/// ```
/// use project6::code::encode_c_instruction;
/// let instruction = encode_c_instruction("D", "D+1", "");
/// assert_eq!(instruction, "1110011111010000");
/// ```
#[inline]
#[must_use]
pub fn encode_c_instruction(
    dest_mnemonic: &str,
    comp_mnemonic: &str,
    jump_mnemonic: &str,
) -> String {
    format!(
        "111{}{}{}",
        comp(comp_mnemonic),
        dest(dest_mnemonic),
        jump(jump_mnemonic)
    )
}

/// Encodes an A-instruction
///
/// A-instruction format: 0vvvvvvvvvvvvvvv (16 bits)
/// - 0: A-instruction prefix (1 bit)
/// - vvvvvvvvvvvvvvv: 15-bit address/value
///
/// # Arguments
/// * `address` - 15-bit address value (0-32767)
///
/// # Returns
/// 16-bit binary string
///
/// # Example
/// ```
/// use project6::code::encode_a_instruction;
/// let instruction = encode_a_instruction(100);
/// assert_eq!(instruction, "0000000001100100");
/// ```
#[inline]
#[must_use]
pub fn encode_a_instruction(address: u16) -> String {
    format!("{address:016b}")
}

/// Validates mnemonics for all three parts of a C-instruction
///
/// Useful for error checking and validation.
///
/// # Arguments
/// * `dest_mnemonic` - Destination mnemonic to validate
/// * `comp_mnemonic` - Computation mnemonic to validate
/// * `jump_mnemonic` - Jump mnemonic to validate
///
/// # Returns
/// Tuple of (`dest_valid`, `comp_valid`, `jump_valid`)
///
/// # Example
/// ```
/// use project6::code::validate_mnemonics;
/// let (d, c, j) = validate_mnemonics("D", "D+1", "JMP");
/// assert!(d && c && j);
/// ```
#[inline]
#[must_use]
#[allow(dead_code)] // Public API function used in tests and documentation
pub fn validate_mnemonics(
    dest_mnemonic: &str,
    comp_mnemonic: &str,
    jump_mnemonic: &str,
) -> (bool, bool, bool) {
    (
        DEST_MAP.contains_key(dest_mnemonic),
        COMP_MAP.contains_key(comp_mnemonic),
        JUMP_MAP.contains_key(jump_mnemonic),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dest_translations() {
        assert_eq!(dest(""), "000");
        assert_eq!(dest("M"), "001");
        assert_eq!(dest("D"), "010");
        assert_eq!(dest("MD"), "011");
        assert_eq!(dest("A"), "100");
        assert_eq!(dest("AM"), "101");
        assert_eq!(dest("AD"), "110");
        assert_eq!(dest("AMD"), "111");

        // Invalid mnemonic should return default
        assert_eq!(dest("INVALID"), "000");
    }

    #[test]
    fn test_comp_translations() {
        // a=0 cases
        assert_eq!(comp("0"), "0101010");
        assert_eq!(comp("1"), "0111111");
        assert_eq!(comp("D"), "0001100");
        assert_eq!(comp("A"), "0110000");
        assert_eq!(comp("D+A"), "0000010");
        assert_eq!(comp("D&A"), "0000000");

        // a=1 cases
        assert_eq!(comp("M"), "1110000");
        assert_eq!(comp("D+M"), "1000010");
        assert_eq!(comp("D&M"), "1000000");

        // Invalid mnemonic should return default
        assert_eq!(comp("INVALID"), "0101010");
    }

    #[test]
    fn test_jump_translations() {
        assert_eq!(jump(""), "000");
        assert_eq!(jump("JGT"), "001");
        assert_eq!(jump("JEQ"), "010");
        assert_eq!(jump("JGE"), "011");
        assert_eq!(jump("JLT"), "100");
        assert_eq!(jump("JNE"), "101");
        assert_eq!(jump("JLE"), "110");
        assert_eq!(jump("JMP"), "111");

        // Invalid mnemonic should return default
        assert_eq!(jump("INVALID"), "000");
    }

    #[test]
    fn test_encode_c_instruction() {
        // D=D+1
        assert_eq!(encode_c_instruction("D", "D+1", ""), "1110011111010000");

        // MD=M-1;JEQ
        assert_eq!(encode_c_instruction("MD", "M-1", "JEQ"), "1111110010011010");

        // 0;JMP (unconditional jump)
        assert_eq!(encode_c_instruction("", "0", "JMP"), "1110101010000111");

        // M=1
        assert_eq!(encode_c_instruction("M", "1", ""), "1110111111001000");
    }

    #[test]
    fn test_encode_a_instruction() {
        assert_eq!(encode_a_instruction(0), "0000000000000000");
        assert_eq!(encode_a_instruction(100), "0000000001100100");
        assert_eq!(encode_a_instruction(16384), "0100000000000000");
        assert_eq!(encode_a_instruction(32767), "0111111111111111");
    }

    #[test]
    fn test_validate_mnemonics() {
        // All valid
        let (d, c, j) = validate_mnemonics("D", "D+1", "JMP");
        assert!(d && c && j);

        // Invalid dest
        let (d, c, j) = validate_mnemonics("INVALID", "D+1", "JMP");
        assert!(!d && c && j);

        // Empty strings are valid (null dest/jump, "0" comp)
        let (d, c, j) = validate_mnemonics("", "0", "");
        assert!(d && c && j);
    }

    #[test]
    fn test_all_dest_mnemonics() {
        // Test that all 8 dest combinations work
        let dest_mnemonics = ["", "M", "D", "MD", "A", "AM", "AD", "AMD"];
        for mnemonic in &dest_mnemonics {
            let result = dest(mnemonic);
            assert_eq!(result.len(), 3);
            assert!(result.chars().all(|c| c == '0' || c == '1'));
        }
    }

    #[test]
    fn test_all_jump_mnemonics() {
        // Test that all 8 jump combinations work
        let jump_mnemonics = ["", "JGT", "JEQ", "JGE", "JLT", "JNE", "JLE", "JMP"];
        for mnemonic in &jump_mnemonics {
            let result = jump(mnemonic);
            assert_eq!(result.len(), 3);
            assert!(result.chars().all(|c| c == '0' || c == '1'));
        }
    }
}
