//! High-Performance Assembler Benchmarks
//!
//! Comprehensive benchmark suite measuring:
//! - Code lookup performance (PHF maps)
//! - Parser throughput
//! - Symbol table operations (FxHashMap)
//! - Full assembly pipeline
//!
//! Run with:
//! ```bash
//! cargo bench
//! cargo bench --bench assembler_bench -- --save-baseline master
//! ```

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use project6::{code, parser::ParserLines, symbol_table::SymbolTable};

/// Benchmark: PHF-based code lookups (O(1) compilation-time perfect hash)
fn bench_code_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("code_lookups");
    group.throughput(Throughput::Elements(4));

    // Test dest lookup performance
    group.bench_function("dest_lookup_hot", |b| {
        b.iter(|| {
            black_box(code::dest("AMD"));
            black_box(code::dest("D"));
            black_box(code::dest("M"));
            black_box(code::dest(""));
        });
    });

    // Test comp lookup performance (most complex)
    group.bench_function("comp_lookup_hot", |b| {
        b.iter(|| {
            black_box(code::comp("D+1"));
            black_box(code::comp("D&M"));
            black_box(code::comp("M-D"));
            black_box(code::comp("0"));
        });
    });

    // Test jump lookup performance
    group.bench_function("jump_lookup_hot", |b| {
        b.iter(|| {
            black_box(code::jump("JMP"));
            black_box(code::jump("JEQ"));
            black_box(code::jump(""));
        });
    });

    // Combined C-instruction encoding (hot path)
    group.bench_function("encode_c_instruction_hot", |b| {
        b.iter(|| {
            black_box(code::encode_c_instruction("D", "D+1", "JMP"));
        });
    });

    // Test cache miss scenario (less common mnemonics)
    group.bench_function("encode_c_instruction_cold", |b| {
        b.iter(|| {
            black_box(code::encode_c_instruction("AMD", "D|M", "JLE"));
        });
    });

    group.finish();
}

/// Benchmark: A-instruction encoding (binary conversion)
fn bench_a_instruction(c: &mut Criterion) {
    let mut group = c.benchmark_group("a_instruction");

    group.bench_function("encode_small_address", |b| {
        b.iter(|| black_box(code::encode_a_instruction(100)));
    });

    group.bench_function("encode_large_address", |b| {
        b.iter(|| black_box(code::encode_a_instruction(16384)));
    });

    group.bench_function("encode_max_address", |b| {
        b.iter(|| black_box(code::encode_a_instruction(32767)));
    });

    // Batch encoding test
    group.throughput(Throughput::Elements(100));
    group.bench_function("encode_batch_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(code::encode_a_instruction(i));
            }
        });
    });

    group.finish();
}

/// Benchmark: Symbol table operations (FxHashMap performance)
fn bench_symbol_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_table");

    // Initialization with all predefined symbols
    group.bench_function("initialization", |b| {
        b.iter(|| black_box(SymbolTable::new()));
    });

    // Lookup predefined symbols (cache-hot scenario)
    group.throughput(Throughput::Elements(3));
    group.bench_function("lookup_predefined_hot", |b| {
        let table = SymbolTable::new();
        b.iter(|| {
            black_box(table.get_address("SP"));
            black_box(table.get_address("R15"));
            black_box(table.get_address("SCREEN"));
        });
    });

    // Insert performance (amortized O(1))
    group.bench_function("insert_sequential", |b| {
        let mut counter = 0;
        b.iter(|| {
            let mut table = SymbolTable::new();
            for i in 0..10 {
                table.add_entry(&format!("VAR{}", counter + i), 16 + i);
            }
            counter += 10;
            black_box(table);
        });
    });

    // Contains check performance
    group.bench_function("contains_check", |b| {
        let table = SymbolTable::new();
        b.iter(|| {
            black_box(table.contains("SP"));
            black_box(table.contains("NONEXISTENT"));
        });
    });

    // HOT PATH: get_or_insert (most common operation in pass 2)
    group.bench_function("get_or_insert_existing", |b| {
        let mut table = SymbolTable::new();
        table.add_entry("LOOP", 100);
        let mut ram_address = 16;
        b.iter(|| {
            black_box(table.get_or_insert("LOOP", &mut ram_address));
        });
    });

    group.bench_function("get_or_insert_new", |b| {
        let mut counter = 0;
        b.iter(|| {
            let mut table = SymbolTable::new();
            let mut ram_address = 16;
            black_box(table.get_or_insert(&format!("VAR{}", counter), &mut ram_address));
            counter += 1;
        });
    });

    group.finish();
}

/// Benchmark: Parser performance (byte-level optimized)
fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // Test data
    let lines_a: Vec<String> = vec!["@100".to_string(), "@SP".to_string(), "@SCREEN".to_string()];

    let lines_c: Vec<String> = vec![
        "D=D+1".to_string(),
        "MD=M-1;JEQ".to_string(),
        "0;JMP".to_string(),
    ];

    let lines_mixed: Vec<String> = vec![
        "@100".to_string(),
        "D=A".to_string(),
        "(LOOP)".to_string(),
        "D=D-1".to_string(),
        "@LOOP".to_string(),
        "D;JGT".to_string(),
    ];

    // A-command parsing (optimized with unsafe get_unchecked)
    group.throughput(Throughput::Elements(lines_a.len() as u64));
    group.bench_function("parse_a_commands_optimized", |b| {
        b.iter(|| {
            let mut parser = ParserLines::from_lines(&lines_a);
            while parser.advance() {
                black_box(parser.command_type().unwrap());
                black_box(parser.symbol().unwrap());
            }
        });
    });

    // C-command parsing (hot path - most complex)
    group.throughput(Throughput::Elements(lines_c.len() as u64));
    group.bench_function("parse_c_commands_full", |b| {
        b.iter(|| {
            let mut parser = ParserLines::from_lines(&lines_c);
            while parser.advance() {
                black_box(parser.command_type().unwrap());
                black_box(parser.dest().unwrap());
                black_box(parser.comp().unwrap());
                black_box(parser.jump().unwrap());
            }
        });
    });

    // Mixed commands (realistic scenario)
    group.throughput(Throughput::Elements(lines_mixed.len() as u64));
    group.bench_function("parse_mixed_realistic", |b| {
        b.iter(|| {
            let mut parser = ParserLines::from_lines(&lines_mixed);
            while parser.advance() {
                let cmd_type = parser.command_type().unwrap();
                black_box(cmd_type);
            }
        });
    });

    // Comment stripping (byte-level optimization test)
    let lines_with_comments: Vec<String> = vec![
        "@100 // This is a comment".to_string(),
        "D=D+1 // Another comment".to_string(),
        "// Full line comment".to_string(),
        "   // Indented comment".to_string(),
        "M=M+1".to_string(),
    ];

    group.throughput(Throughput::Elements(lines_with_comments.len() as u64));
    group.bench_function("parse_comments_bytescan", |b| {
        b.iter(|| {
            let mut parser = ParserLines::from_lines(&lines_with_comments);
            while parser.advance() {
                black_box(parser.command_type().unwrap());
            }
        });
    });

    // Whitespace handling performance
    let lines_whitespace: Vec<String> = vec![
        "   @100   ".to_string(),
        "  D=M  ".to_string(),
        "\t\tM=D+1\t\t".to_string(),
    ];

    group.bench_function("parse_whitespace_trim", |b| {
        b.iter(|| {
            let mut parser = ParserLines::from_lines(&lines_whitespace);
            while parser.advance() {
                black_box(parser.command_type().unwrap());
            }
        });
    });

    group.finish();
}

/// Benchmark: Full assembly pipeline (end-to-end performance)
fn bench_full_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_assembly");

    // Small program: Simple addition
    let small_program: Vec<String> = vec![
        "@2".to_string(),
        "D=A".to_string(),
        "@3".to_string(),
        "D=D+A".to_string(),
        "@0".to_string(),
        "M=D".to_string(),
    ];

    // Medium program: Loop with variables
    let medium_program: Vec<String> = vec![
        "@100".to_string(),
        "D=A".to_string(),
        "@i".to_string(),
        "M=D".to_string(),
        "(LOOP)".to_string(),
        "@i".to_string(),
        "D=M".to_string(),
        "@END".to_string(),
        "D;JEQ".to_string(),
        "@i".to_string(),
        "M=M-1".to_string(),
        "@LOOP".to_string(),
        "0;JMP".to_string(),
        "(END)".to_string(),
        "@END".to_string(),
        "0;JMP".to_string(),
    ];

    // Realistic program: Multiple functions
    let realistic_program: Vec<String> = vec![
        "// Initialize".to_string(),
        "@256".to_string(),
        "D=A".to_string(),
        "@SP".to_string(),
        "M=D".to_string(),
        "(MAIN)".to_string(),
        "@10".to_string(),
        "D=A".to_string(),
        "@sum".to_string(),
        "M=D".to_string(),
        "(LOOP_START)".to_string(),
        "@sum".to_string(),
        "D=M".to_string(),
        "@LOOP_END".to_string(),
        "D;JEQ".to_string(),
        "@sum".to_string(),
        "M=M-1".to_string(),
        "@LOOP_START".to_string(),
        "0;JMP".to_string(),
        "(LOOP_END)".to_string(),
        "@LOOP_END".to_string(),
        "0;JMP".to_string(),
    ];

    group.throughput(Throughput::Elements(small_program.len() as u64));
    group.bench_function("pipeline_small_6_lines", |b| {
        b.iter(|| assemble_program(black_box(&small_program)));
    });

    group.throughput(Throughput::Elements(medium_program.len() as u64));
    group.bench_function("pipeline_medium_16_lines", |b| {
        b.iter(|| assemble_program(black_box(&medium_program)));
    });

    group.throughput(Throughput::Elements(realistic_program.len() as u64));
    group.bench_function("pipeline_realistic_21_lines", |b| {
        b.iter(|| assemble_program(black_box(&realistic_program)));
    });

    // Large program stress test (160 lines)
    let large_program: Vec<String> = medium_program
        .iter()
        .cycle()
        .take(medium_program.len() * 10)
        .cloned()
        .collect();

    group.throughput(Throughput::Elements(large_program.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("pipeline_large", large_program.len()),
        &large_program,
        |b, prog| {
            b.iter(|| assemble_program(black_box(prog)));
        },
    );

    group.finish();
}

/// Helper: Complete assembly process simulation
fn assemble_program(lines: &[String]) -> Vec<String> {
    let mut output = Vec::new();
    let mut symbol_table = SymbolTable::new();
    let mut rom_address = 0;

    // First pass
    let mut parser = ParserLines::from_lines(lines);
    while parser.advance() {
        match parser.command_type().unwrap() {
            project6::CommandType::LCommand => {
                symbol_table.add_entry(parser.symbol().unwrap(), rom_address);
            }
            _ => {
                rom_address += 1;
            }
        }
    }

    // Second pass
    let mut ram_address = 16;
    let mut parser = ParserLines::from_lines(lines);
    while parser.advance() {
        match parser.command_type().unwrap() {
            project6::CommandType::ACommand => {
                let symbol = parser.symbol().unwrap();
                let address = symbol
                    .parse::<u16>()
                    .unwrap_or_else(|_| symbol_table.get_or_insert(symbol, &mut ram_address));
                output.push(code::encode_a_instruction(address));
            }
            project6::CommandType::CCommand => {
                let instruction = code::encode_c_instruction(
                    parser.dest().unwrap().unwrap_or(""),
                    parser.comp().unwrap().unwrap_or(""),
                    parser.jump().unwrap().unwrap_or(""),
                );
                output.push(instruction);
            }
            project6::CommandType::LCommand => {}
        }
    }

    output
}

/// Benchmark: Low-level string operations
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // Comment detection: byte scan vs find
    group.bench_function("comment_bytescan", |b| {
        let line = "@100 // This is a comment";
        b.iter(|| {
            let bytes = line.as_bytes();
            for i in 0..bytes.len().saturating_sub(1) {
                if bytes[i] == b'/' && bytes[i + 1] == b'/' {
                    black_box(i);
                    break;
                }
            }
        });
    });

    group.bench_function("comment_find", |b| {
        let line = "@100 // This is a comment";
        b.iter(|| {
            black_box(line.find("//"));
        });
    });

    // Character search performance
    group.bench_function("find_equals_bytescan", |b| {
        let line = "D=D+1";
        let bytes = line.as_bytes();
        b.iter(|| {
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'=' {
                    black_box(i);
                    break;
                }
            }
        });
    });

    group.bench_function("find_equals_std", |b| {
        let line = "D=D+1";
        b.iter(|| {
            black_box(line.find('='));
        });
    });

    // Unsafe slice vs safe slice
    group.bench_function("slice_safe", |b| {
        let line = "@VARIABLE";
        b.iter(|| {
            black_box(&line[1..]);
        });
    });

    group.bench_function("slice_unsafe", |b| {
        let line = "@VARIABLE";
        b.iter(|| {
            black_box(unsafe { line.get_unchecked(1..) });
        });
    });

    // Whitespace trimming
    group.bench_function("trim_std", |b| {
        let line = "   @100   ";
        b.iter(|| {
            black_box(line.trim());
        });
    });

    group.bench_function("trim_manual_bytes", |b| {
        let line = "   @100   ";
        b.iter(|| {
            let bytes = line.as_bytes();
            let mut start = 0;
            let mut end = bytes.len();
            while start < end && bytes[start].is_ascii_whitespace() {
                start += 1;
            }
            while end > start && bytes[end - 1].is_ascii_whitespace() {
                end -= 1;
            }
            black_box((start, end));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_code_lookups,
    bench_a_instruction,
    bench_symbol_table,
    bench_parser,
    bench_full_assembly,
    bench_string_operations,
);

criterion_main!(benches);
