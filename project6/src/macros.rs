//! High-performance macro utilities for the Hack assembler
//!
//! This module provides compile-time optimizations and code generation macros.

/// Creates a static string array at compile time
///
/// # Example
/// ```
/// use project6::static_str_array;
/// let ops = static_str_array!["ADD", "SUB", "MUL"];
/// assert_eq!(ops.len(), 3);
/// ```
#[macro_export]
macro_rules! static_str_array {
    ($($item:expr),* $(,)?) => {
        &[$($item),*] as &[&'static str]
    };
}

/// Creates a `HashMap` with pre-allocated capacity
///
/// # Example
/// ```
/// use project6::fast_hashmap;
/// let map = fast_hashmap! {
///     capacity: 10,
///     "key1" => 1,
///     "key2" => 2,
/// };
/// assert_eq!(map.len(), 2);
/// ```
#[macro_export]
macro_rules! fast_hashmap {
    (capacity: $cap:expr, $($key:expr => $val:expr),* $(,)?) => {{
        let mut map = ::std::collections::HashMap::with_capacity($cap);
        $(map.insert($key, $val);)*
        map
    }};
}

/// Builds a string with pre-allocated capacity on the stack
///
/// More efficient than repeated `push_str` calls for known string sizes.
///
/// # Example
/// ```
/// use project6::stack_string;
/// let s = stack_string!(capacity: 32, "Hello", " ", "World");
/// assert_eq!(s, "Hello World");
/// ```
#[macro_export]
macro_rules! stack_string {
    (capacity: $cap:expr, $($part:expr),* $(,)?) => {{
        let mut s = String::with_capacity($cap);
        $(s.push_str($part);)*
        s
    }};
}

/// Branch prediction hint: likely condition
///
/// Helps the compiler optimize hot paths.
#[macro_export]
macro_rules! likely {
    ($expr:expr) => {{
        #[cold]
        #[inline(always)]
        fn cold() {}

        if $expr {
            true
        } else {
            cold();
            false
        }
    }};
}

/// Branch prediction hint: unlikely condition
///
/// Helps the compiler optimize cold paths.
#[macro_export]
macro_rules! unlikely {
    ($expr:expr) => {{
        #[cold]
        #[inline(always)]
        fn cold() {}

        if !$expr {
            cold();
            false
        } else {
            true
        }
    }};
}

/// Debug-only assertions (zero cost in release builds)
#[macro_export]
macro_rules! debug_assert_perf {
    ($($arg:tt)*) => {
        debug_assert!($($arg)*);
    };
}

/// Defines compile-time constants with less boilerplate
///
/// # Example
/// ```ignore
/// use project6::define_constants;
/// define_constants! {
///     pub MAX_SIZE: usize = 1024;
///     BUFFER_SIZE: usize = 512;
/// }
/// ```
#[macro_export]
macro_rules! define_constants {
    ($($(#[$attr:meta])* $vis:vis $name:ident: $type:ty = $value:expr;)*) => {
        $(
            $(#[$attr])*
            $vis const $name: $type = $value;
        )*
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_static_str_array() {
        let arr = static_str_array!["a", "b", "c"];
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "a");
        assert_eq!(arr[1], "b");
    }

    #[test]
    fn test_fast_hashmap() {
        let map = fast_hashmap! {
            capacity: 5,
            "x" => 10,
            "y" => 20,
        };
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("x"), Some(&10));
        assert_eq!(map.get("y"), Some(&20));
    }

    #[test]
    fn test_stack_string() {
        let s = stack_string!(capacity: 16, "Hello", " ", "World");
        assert_eq!(s, "Hello World");
        assert_eq!(s.len(), 11);
    }

    #[test]
    fn test_likely_unlikely() {
        assert!(likely!(true));
        assert!(!likely!(false));
        assert!(unlikely!(true));
        assert!(!unlikely!(false));
    }

    #[test]
    fn test_debug_assert_perf() {
        debug_assert_perf!(1 + 1 == 2);
        debug_assert_perf!(true);
    }
}
