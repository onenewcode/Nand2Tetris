// vm command:push constant 111
@111
D=A
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:push constant 333
@333
D=A
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:push constant 888
@888
D=A
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:pop static 8
// get the top element of stack
@SP
M=M-1
A=M
D=M
@StaticTest.temp.8
M=D

// vm command:pop static 3
// get the top element of stack
@SP
M=M-1
A=M
D=M
@StaticTest.temp.3
M=D

// vm command:pop static 1
// get the top element of stack
@SP
M=M-1
A=M
D=M
@StaticTest.temp.1
M=D

// vm command:push static 3
@StaticTest.temp.3
D=M
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:push static 1
@StaticTest.temp.1
D=M
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:sub
// get the top element of stack
@SP
M=M-1
A=M
D=M
// store the result temporarily
@R14
M=D
// get the top element of stack
@SP
M=M-1
A=M
D=M
// store the result temporarily
@R13
M=D
@R13
D=M
@R14
D=D-M
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:push static 8
@StaticTest.temp.8
D=M
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

// vm command:add
// get the top element of stack
@SP
M=M-1
A=M
D=M
// store the result temporarily
@R14
M=D
// get the top element of stack
@SP
M=M-1
A=M
D=M
// store the result temporarily
@R13
M=D
@R13
D=M
@R14
D=D+M
// push the value into stack
@SP
A=M
M=D
@SP
M=M+1

