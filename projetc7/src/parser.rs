use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Arithmetic,
    Push,
    Pop,
    Label,
    Goto,
    If,
    Function,
    Return,
    Call,
}

pub struct Parser {
    lines: Vec<String>,
    current_line: usize,
    current_command: String,
    /// Cached parts of the current command to avoid repeated parsing
    cached_parts: Vec<String>,
}

impl Parser {
    pub fn new(filename: &str) -> Result<Self, std::io::Error> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);

        let mut lines = Vec::new();
        for line in reader.lines() {
            let line = line?;
            // Remove comments and whitespace
            let line = if let Some(pos) = line.find("//") {
                &line[..pos]
            } else {
                &line
            };

            let trimmed = line.trim();

            // Skip empty lines
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }

        Ok(Parser {
            lines,
            current_line: 0,
            current_command: String::new(),
            cached_parts: Vec::new(),
        })
    }

    #[inline]
    pub fn has_more_commands(&self) -> bool {
        self.current_line < self.lines.len()
    }

    #[inline]
    pub fn advance(&mut self) {
        if self.has_more_commands() {
            // Use swap to avoid allocation
            std::mem::swap(
                &mut self.current_command,
                &mut self.lines[self.current_line],
            );

            // Parse and cache command parts once
            self.cached_parts.clear();
            self.cached_parts.extend(
                self.current_command
                    .split_whitespace()
                    .map(|s| s.to_string()),
            );

            self.current_line += 1;
        }
    }

    #[inline]
    pub fn command_type(&self) -> CommandType {
        debug_assert!(!self.cached_parts.is_empty(), "Empty command");

        match self.cached_parts[0].as_str() {
            "push" => CommandType::Push,
            "pop" => CommandType::Pop,
            "label" => CommandType::Label,
            "goto" => CommandType::Goto,
            "if-goto" => CommandType::If,
            "function" => CommandType::Function,
            "return" => CommandType::Return,
            "call" => CommandType::Call,
            _ => CommandType::Arithmetic,
        }
    }

    #[inline]
    pub fn arg1(&self) -> &str {
        let cmd_type = self.command_type();
        match cmd_type {
            CommandType::Arithmetic => &self.cached_parts[0],
            CommandType::Return => panic!("arg1 should not be called for Return"),
            _ => {
                debug_assert!(self.cached_parts.len() > 1, "No arg1 found");
                &self.cached_parts[1]
            }
        }
    }

    #[inline]
    pub fn arg2(&self) -> i32 {
        let cmd_type = self.command_type();
        match cmd_type {
            CommandType::Push | CommandType::Pop | CommandType::Function | CommandType::Call => {
                debug_assert!(self.cached_parts.len() > 2, "No arg2 found");
                self.cached_parts[2].parse().expect("Invalid arg2")
            }
            _ => panic!("arg2 should not be called for this command type"),
        }
    }
}
