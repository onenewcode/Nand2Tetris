use std::fs::File;
use std::io::{BufWriter, Write};

// 定义一个宏来简化汇编代码的写入
macro_rules! write_asm {
    ($writer:expr, $($line:literal)*) => {
        $writer.write_all(concat!($($line, "\n"),*).as_bytes())
    };
}

#[derive(Clone, Copy)]
enum SegmentSymbol {
    Local,
    Argument,
    This,
    That,
    Temp,
    Pointer,
    Static,
    Constant,
}

impl SegmentSymbol {
    fn from_str(segment: &str) -> Option<Self> {
        match segment {
            "local" => Some(SegmentSymbol::Local),
            "argument" => Some(SegmentSymbol::Argument),
            "this" => Some(SegmentSymbol::This),
            "that" => Some(SegmentSymbol::That),
            "temp" => Some(SegmentSymbol::Temp),
            "pointer" => Some(SegmentSymbol::Pointer),
            "static" => Some(SegmentSymbol::Static),
            "constant" => Some(SegmentSymbol::Constant),
            _ => None,
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            SegmentSymbol::Local => "LCL",
            SegmentSymbol::Argument => "ARG",
            SegmentSymbol::This => "THIS",
            SegmentSymbol::That => "THAT",
            SegmentSymbol::Temp => "R5",
            SegmentSymbol::Pointer => "THIS", // Special case handled separately
            SegmentSymbol::Static => "STATIC", // Special case handled separately
            SegmentSymbol::Constant => "CONSTANT", // Special case handled separately
        }
    }
}

pub struct CodeWriter {
    output_file: BufWriter<File>,
    label_counter: usize,
    filename: String,
}

impl CodeWriter {
    /// 创建一个新的CodeWriter实例，用于将汇编代码写入指定的输出文件，默认启动使用Buf占据8192字节。
    pub fn new(output_filename: &str) -> Result<Self, std::io::Error> {
        let file = File::create(output_filename)?;
        let buffered = BufWriter::with_capacity(8192, file);
        Ok(CodeWriter {
            output_file: buffered,
            label_counter: 0,
            filename: String::new(),
        })
    }

    #[inline]
    pub fn set_filename(&mut self, filename: &str) {
        // Extract filename without path and extension
        let name = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");
        self.filename.clear();
        self.filename.push_str(name);
    }

    pub fn write_arithmetic(&mut self, command: &str) -> Result<(), std::io::Error> {
        writeln!(self.output_file, "// vm command:{}", command)?;

        match command {
            "add" => self.write_binary_op("D+M"),
            "sub" => self.write_binary_op("D-M"),
            "and" => self.write_binary_op("D&M"),
            "or" => self.write_binary_op("D|M"),
            "neg" => self.write_unary_op(true),
            "not" => self.write_unary_op(false),
            "eq" => self.write_comparison("JEQ"),
            "gt" => self.write_comparison("JGT"),
            "lt" => self.write_comparison("JLT"),
            _ => panic!("Unknown arithmetic command: {}", command),
        }
    }

    #[inline]
    fn write_binary_op(&mut self, operation: &str) -> Result<(), std::io::Error> {
        // Optimized: write all at once to reduce syscalls
        write!(
            self.output_file,
            "// get the top element of stack\n\
             @SP\n\
             M=M-1\n\
             A=M\n\
             D=M\n\
             // store the result temporarily\n\
             @R14\n\
             M=D\n\
             // get the top element of stack\n\
             @SP\n\
             M=M-1\n\
             A=M\n\
             D=M\n\
             // store the result temporarily\n\
             @R13\n\
             M=D\n\
             @R13\n\
             D=M\n\
             @R14\n\
             D={}\n",
            operation
        )?;

        self.write_push_d()?;
        self.output_file.write_all(b"\n")?;
        Ok(())
    }

    #[inline]
    fn write_unary_op(&mut self, is_neg: bool) -> Result<(), std::io::Error> {
        write_asm!(self.output_file,
            "// get the top element of stack"
            "@SP"
            "M=M-1"
            "A=M"
            "D=M"
        )?;

        if is_neg {
            write_asm!(self.output_file,
                "@0"
                "D=A-D"
            )?;
        } else {
            write_asm!(self.output_file, "D=!D")?;
        }

        self.write_push_d()?;
        self.output_file.write_all(b"\n")?;
        Ok(())
    }

    #[inline]
    fn write_comparison(&mut self, jump: &str) -> Result<(), std::io::Error> {
        let label_prefix = match jump {
            "JEQ" => "EQ",
            "JGT" => "GT",
            "JLT" => "LT",
            _ => jump,
        };
        let label_num = self.label_counter;
        self.label_counter += 1;

        write!(
            self.output_file,
            "// get the top element of stack\n\
             @SP\n\
             M=M-1\n\
             A=M\n\
             D=M\n\
             // store the result temporarily\n\
             @R14\n\
             M=D\n\
             // get the top element of stack\n\
             @SP\n\
             M=M-1\n\
             A=M\n\
             D=M\n\
             // store the result temporarily\n\
             @R13\n\
             M=D\n\
             @R13\n\
             D=M\n\
             @R14\n\
             D=D-M\n\
             @{}{}\n\
             D;{}\n\
             // push the value into stack\n\
             @SP\n\
             A=M\n\
             M=0\n\
             @SP\n\
             M=M+1\n\
             @END{}{}\n\
             0;JMP\n\
             ({}{})\n\
             // push the value into stack\n\
             @SP\n\
             A=M\n\
             M=-1\n\
             @SP\n\
             M=M+1\n\
             (END{}{})\n\n",
            label_prefix,
            label_num,
            jump,
            label_prefix,
            label_num,
            label_prefix,
            label_num,
            label_prefix,
            label_num
        )
    }

    pub fn write_push_pop(
        &mut self,
        command: &str,
        segment: &str,
        index: i32,
    ) -> Result<(), std::io::Error> {
        writeln!(
            self.output_file,
            "// vm command:{} {} {}",
            command, segment, index
        )?;

        if command == "push" {
            self.write_push(segment, index)?;
        } else if command == "pop" {
            self.write_pop(segment, index)?;
        }

        self.output_file.write_all(b"\n")?;
        Ok(())
    }

    #[inline]
    fn write_push(&mut self, segment: &str, index: i32) -> Result<(), std::io::Error> {
        match SegmentSymbol::from_str(segment) {
            Some(SegmentSymbol::Constant) => {
                write!(self.output_file, "@{}\nD=A\n", index)?;
                self.write_push_d()
            }
            Some(seg)
                if matches!(
                    seg,
                    SegmentSymbol::Local
                        | SegmentSymbol::Argument
                        | SegmentSymbol::This
                        | SegmentSymbol::That
                ) =>
            {
                let segment_symbol = seg.symbol();
                write!(
                    self.output_file,
                    "@{}\nD=M\n@{}\nA=D+A\nD=M\n",
                    segment_symbol, index
                )?;
                self.write_push_d()
            }
            Some(SegmentSymbol::Temp) => {
                write!(self.output_file, "@R5\nD=A\n@{}\nA=D+A\nD=M\n", index)?;
                self.write_push_d()
            }
            Some(SegmentSymbol::Pointer) => {
                write!(self.output_file, "@THIS\nD=A\n@{}\nA=D+A\nD=M\n", index)?;
                self.write_push_d()
            }
            Some(SegmentSymbol::Static) => {
                write!(self.output_file, "@{}.{}\nD=M\n", self.filename, index)?;
                self.write_push_d()
            }
            _ => panic!("Unknown segment: {}", segment),
        }
    }

    #[inline]
    fn write_pop(&mut self, segment: &str, index: i32) -> Result<(), std::io::Error> {
        match SegmentSymbol::from_str(segment) {
            Some(seg)
                if matches!(
                    seg,
                    SegmentSymbol::Local
                        | SegmentSymbol::Argument
                        | SegmentSymbol::This
                        | SegmentSymbol::That
                ) =>
            {
                let segment_symbol = seg.symbol();
                write!(
                    self.output_file,
                    "@{}\n\
                     D=M\n\
                     @{}\n\
                     D=D+A\n\
                     // store the result temporarily\n\
                     @R13\n\
                     M=D\n",
                    segment_symbol, index
                )?;

                self.write_pop_to_d()?;

                write_asm!(self.output_file,
                    "// store the top value"
                    "@R13"
                    "A=M"
                    "M=D"
                )?;
                Ok(())
            }
            Some(SegmentSymbol::Temp) => {
                write!(
                    self.output_file,
                    "@5\n\
                     D=A\n\
                     @{}\n\
                     D=D+A\n\
                     // store the result temporarily\n\
                     @R13\n\
                     M=D\n",
                    index
                )?;

                self.write_pop_to_d()?;

                write_asm!(self.output_file,
                    "// store the top value"
                    "@R13"
                    "A=M"
                    "M=D"
                )?;
                Ok(())
            }
            Some(SegmentSymbol::Pointer) => {
                write!(
                    self.output_file,
                    "@THIS\n\
                     D=A\n\
                     @{}\n\
                     D=D+A\n\
                     // store the result temporarily\n\
                     @R13\n\
                     M=D\n",
                    index
                )?;

                self.write_pop_to_d()?;

                write_asm!(self.output_file,
                    "// store the top value"
                    "@R13"
                    "A=M"
                    "M=D"
                )?;
                Ok(())
            }
            Some(SegmentSymbol::Static) => {
                self.write_pop_to_d()?;
                write!(self.output_file, "@{}.{}\nM=D\n", self.filename, index)
            }
            _ => panic!("Cannot pop to segment: {}", segment),
        }
    }

    #[inline]
    fn write_push_d(&mut self) -> Result<(), std::io::Error> {
        write_asm!(self.output_file,
            "// push the value into stack"
            "@SP"
            "A=M"
            "M=D"
            "@SP"
            "M=M+1"
        )
    }

    #[inline]
    fn write_pop_to_d(&mut self) -> Result<(), std::io::Error> {
        write_asm!(self.output_file,
            "// get the top element of stack"
            "@SP"
            "M=M-1"
            "A=M"
            "D=M"
        )
    }

    #[inline]
    pub fn close(&mut self) -> Result<(), std::io::Error> {
        self.output_file.flush()
    }
}
