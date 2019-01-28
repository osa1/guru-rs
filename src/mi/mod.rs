//! GDD machine interface types and parser.

// Input syntax: https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Input-Syntax.html#GDB_002fMI-Input-Syntax
// Output syntax: https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Output-Syntax.html

pub mod commands;
pub mod output_syntax;
pub mod parser;
pub mod syntax_common;

pub use output_syntax::*;
pub use parser::parse_output;
pub use syntax_common::*;
