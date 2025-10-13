//! Symbol table module for Hack assembler
//!
//! Uses a hybrid approach for optimal performance:
//! - PHF (Perfect Hash Function) for predefined symbols - O(1) compile-time lookup
//! - Standard `HashMap` for user-defined symbols - dynamic insertion
//!
//! This gives us the best of both worlds: blazing fast lookups for common symbols
//! and flexibility for user-defined labels and variables.

use phf::phf_map;
use std::collections::HashMap;
use std::fmt;

/// Predefined symbols with compile-time perfect hash
///
/// These symbols are built into the Hack platform and never change.
/// Using PHF gives us zero-cost lookups at runtime.
static PREDEFINED_SYMBOLS: phf::Map<&'static str, u16> = phf_map! {
    // Virtual registers
    "R0" => 0,
    "R1" => 1,
    "R2" => 2,
    "R3" => 3,
    "R4" => 4,
    "R5" => 5,
    "R6" => 6,
    "R7" => 7,
    "R8" => 8,
    "R9" => 9,
    "R10" => 10,
    "R11" => 11,
    "R12" => 12,
    "R13" => 13,
    "R14" => 14,
    "R15" => 15,

    // Special pointers
    "SP" => 0,
    "LCL" => 1,
    "ARG" => 2,
    "THIS" => 3,
    "THAT" => 4,

    // I/O pointers
    "SCREEN" => 16384,
    "KBD" => 24576,
};

/// Symbol table for the Hack assembler
///
/// Maintains mappings between symbolic labels and numeric addresses.
/// Handles both predefined symbols (via PHF) and user-defined symbols (via `HashMap`).
///
/// # Performance Characteristics
/// - Predefined symbol lookup: O(1) compile-time perfect hash
/// - User symbol lookup: O(1) average case `HashMap`
/// - User symbol insertion: O(1) amortized
///
/// # Example
/// ```
/// use project6::SymbolTable;
///
/// let mut st = SymbolTable::new();
///
/// // Predefined symbols are instantly available
/// assert_eq!(st.get_address("SP"), 0);
/// assert_eq!(st.get_address("R15"), 15);
///
/// // User-defined symbols can be added
/// st.add_entry("LOOP", 100);
/// assert_eq!(st.get_address("LOOP"), 100);
/// ```
#[derive(Debug)]
pub struct SymbolTable {
    /// User-defined symbols (labels and variables)
    user_symbols: HashMap<String, u16>,
}

impl Default for SymbolTable {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SymbolTable")
    }
}

impl SymbolTable {
    /// Creates a new symbol table
    ///
    /// Predefined symbols are available via PHF, so no initialization needed.
    /// Pre-allocates space for typical user symbol count (~32 symbols).
    #[must_use]
    pub fn new() -> Self {
        Self {
            user_symbols: HashMap::with_capacity(32),
        }
    }

    /// Adds a user-defined symbol to the table
    ///
    /// # Arguments
    /// * `symbol` - The symbol name
    /// * `address` - The address associated with the symbol
    ///
    /// # Example
    /// ```
    /// use project6::SymbolTable;
    ///
    /// let mut st = SymbolTable::new();
    /// st.add_entry("LOOP", 100);
    /// assert_eq!(st.get_address("LOOP"), 100);
    /// ```
    #[inline]
    pub fn add_entry(&mut self, symbol: &str, address: u16) {
        self.user_symbols.insert(symbol.to_string(), address);
    }

    /// Checks if a symbol exists (either predefined or user-defined)
    ///
    /// # Performance
    /// Fast path: Check PHF first (most common case)
    /// Slow path: Check `HashMap` if not predefined
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Used in tests and public API
    pub fn contains(&self, symbol: &str) -> bool {
        PREDEFINED_SYMBOLS.contains_key(symbol) || self.user_symbols.contains_key(symbol)
    }

    /// Gets the address associated with a symbol
    ///
    /// Returns 0 if the symbol doesn't exist.
    ///
    /// # Performance
    /// Checks predefined symbols first (PHF - O(1) compile-time),
    /// then user symbols (`HashMap` - O(1) average).
    ///
    /// # Example
    /// ```
    /// use project6::SymbolTable;
    ///
    /// let st = SymbolTable::new();
    /// assert_eq!(st.get_address("SP"), 0);
    /// assert_eq!(st.get_address("SCREEN"), 16384);
    /// assert_eq!(st.get_address("NONEXISTENT"), 0);
    /// ```
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Used in tests and public API
    pub fn get_address(&self, symbol: &str) -> u16 {
        // Fast path: Check predefined symbols first (most common)
        if let Some(&addr) = PREDEFINED_SYMBOLS.get(symbol) {
            return addr;
        }

        // Slow path: Check user-defined symbols
        self.user_symbols.get(symbol).copied().unwrap_or(0)
    }

    /// Gets or inserts a symbol, returning its address
    ///
    /// This is the HOT PATH for variable resolution in pass 2.
    /// Optimized for the common case where predefined symbols are checked first.
    ///
    /// # Arguments
    /// * `symbol` - The symbol to look up or insert
    /// * `next_address` - Mutable reference to next available RAM address (auto-incremented on insert)
    ///
    /// # Returns
    /// The address associated with the symbol
    ///
    /// # Example
    /// ```
    /// use project6::SymbolTable;
    ///
    /// let mut st = SymbolTable::new();
    /// let mut ram_addr = 16;
    ///
    /// // First call inserts and returns 16
    /// assert_eq!(st.get_or_insert("var1", &mut ram_addr), 16);
    /// assert_eq!(ram_addr, 17);
    ///
    /// // Second call returns existing address
    /// assert_eq!(st.get_or_insert("var1", &mut ram_addr), 16);
    /// assert_eq!(ram_addr, 17); // Not incremented
    /// ```
    #[inline]
    pub fn get_or_insert(&mut self, symbol: &str, next_address: &mut u16) -> u16 {
        use std::collections::hash_map::Entry;

        // Fast path: Check predefined symbols (most common in well-written code)
        if let Some(&addr) = PREDEFINED_SYMBOLS.get(symbol) {
            return addr;
        }

        // User symbol: use Entry API to avoid double lookup
        match self.user_symbols.entry(symbol.to_string()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let addr = *next_address;
                *next_address += 1;
                *e.insert(addr)
            }
        }
    }

    /// Returns the number of user-defined symbols
    ///
    /// Predefined symbols are not counted as they're stored separately.
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Used in tests and public API
    pub fn user_symbol_count(&self) -> usize {
        self.user_symbols.len()
    }

    /// Returns the total number of predefined symbols (23)
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Used in tests and public API
    pub const fn predefined_symbol_count() -> usize {
        PREDEFINED_SYMBOLS.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predefined_symbols() {
        let st = SymbolTable::new();

        // Special pointers
        assert_eq!(st.get_address("SP"), 0);
        assert_eq!(st.get_address("LCL"), 1);
        assert_eq!(st.get_address("ARG"), 2);
        assert_eq!(st.get_address("THIS"), 3);
        assert_eq!(st.get_address("THAT"), 4);

        // I/O pointers
        assert_eq!(st.get_address("SCREEN"), 16384);
        assert_eq!(st.get_address("KBD"), 24576);

        // Virtual registers
        for i in 0..=15 {
            assert_eq!(st.get_address(&format!("R{i}")), i);
        }
    }

    #[test]
    fn test_add_and_get_user_symbols() {
        let mut st = SymbolTable::new();

        st.add_entry("LOOP", 100);
        assert_eq!(st.get_address("LOOP"), 100);
        assert!(st.contains("LOOP"));

        st.add_entry("END", 200);
        assert_eq!(st.get_address("END"), 200);

        assert!(!st.contains("UNKNOWN"));
        assert_eq!(st.get_address("UNKNOWN"), 0);
    }

    #[test]
    fn test_get_or_insert() {
        let mut st = SymbolTable::new();
        let mut next = 16;

        // First call should insert
        let addr1 = st.get_or_insert("var1", &mut next);
        assert_eq!(addr1, 16);
        assert_eq!(next, 17);

        // Second call should return existing
        let addr2 = st.get_or_insert("var1", &mut next);
        assert_eq!(addr2, 16);
        assert_eq!(next, 17); // Should not increment

        // Third call with new symbol
        let addr3 = st.get_or_insert("var2", &mut next);
        assert_eq!(addr3, 17);
        assert_eq!(next, 18);
    }

    #[test]
    fn test_predefined_not_overwritten() {
        let mut st = SymbolTable::new();
        let mut next = 16;

        // Should return existing predefined address
        let addr = st.get_or_insert("SP", &mut next);
        assert_eq!(addr, 0);
        assert_eq!(next, 16); // Should not increment

        // Should not add to user symbols
        assert_eq!(st.user_symbol_count(), 0);
    }

    #[test]
    fn test_symbol_counts() {
        let mut st = SymbolTable::new();

        assert_eq!(SymbolTable::predefined_symbol_count(), 23);
        assert_eq!(st.user_symbol_count(), 0);

        st.add_entry("LOOP", 100);
        assert_eq!(st.user_symbol_count(), 1);

        st.add_entry("END", 200);
        assert_eq!(st.user_symbol_count(), 2);
    }

    #[test]
    fn test_contains() {
        let mut st = SymbolTable::new();

        // Predefined symbols
        assert!(st.contains("SP"));
        assert!(st.contains("R15"));
        assert!(st.contains("SCREEN"));

        // User symbols
        st.add_entry("LOOP", 100);
        assert!(st.contains("LOOP"));

        // Non-existent
        assert!(!st.contains("NONEXISTENT"));
    }

    #[test]
    fn test_phf_performance() {
        // This test verifies that PHF map is working correctly
        assert_eq!(PREDEFINED_SYMBOLS.get("SP"), Some(&0));
        assert_eq!(PREDEFINED_SYMBOLS.get("R10"), Some(&10));
        assert_eq!(PREDEFINED_SYMBOLS.get("SCREEN"), Some(&16384));
        assert_eq!(PREDEFINED_SYMBOLS.get("INVALID"), None);
    }
}
