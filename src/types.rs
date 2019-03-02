// TODO: We need a type for memory locations

//
// Breakpoint stuff
//

#[derive(Debug)]
pub struct Breakpoint {
    /// Unique number
    pub number: u32,

    pub type_: BreakpointType,

    /// Should the breakpoint be deleted or disabled when it is hit?
    pub disposition: BreakpointDisposition,

    pub enabled: bool,

    /// Memory location at which the breakpoint is set.
    pub address: String,

    /*
        Sigh ... gdb-mi documentation is out of date
        /// Logical location of the breakpoint, expressed by function name, file name, line number.
        pub what: String,
    */
    /*
    /// Function name
    pub func: String,
    */
    pub original_location: String,

    /// File name
    pub file: Option<String>,

    /// Full path of the file
    pub fullname: Option<String>,

    /// Line number
    pub line: Option<u32>,

    /// Condition
    pub cond: Option<String>,

    // TODO thread-groups?
    /// Number of times the breakpoint has been hit
    pub hits: u32,
}

// NOTE: GDB has more details like whether the watchpoint is hardware or not. We ignore those for
// now.
#[derive(Debug, PartialEq, Eq)]
pub enum BreakpointType {
    Breakpoint,
    #[allow(dead_code)]
    Watchpoint,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BreakpointDisposition {
    Keep,
    NoKeep,
}

//
// Watchpoint stuff
//

#[derive(Debug)]
#[allow(dead_code)]
pub struct Watchpoint {
    pub number: u32,
    pub expr: String,
    pub type_: WatchpointType,
}

#[derive(Debug, Copy, Clone)]
pub enum WatchpointType {
    ReadWrite,
    Read,
    Write,
}

//
// Backtrace stuff
//

pub struct Backtrace(pub Vec<Frame>);

#[derive(Debug)]
pub struct Frame {
    pub level: usize,

    /// The $pc value for the frame.
    pub addr: String,

    /// Function name
    pub func: String,

    /// File name of the source file where the function lives.
    pub file: Option<String>,

    /// The full file name of the source file where the function lives.
    pub fullname: Option<String>,

    /// Line number corresponding to the $pc.
    pub line: Option<usize>,

    /// The shared library where this function is defined. This is only given if the frame’s
    /// function is not known.
    pub from: Option<String>,
}

//
// Value/expression stuff
//

#[derive(Debug)]
pub struct Value {
    pub expr: Option<String>, // "exp" field, I don't understand what this is
    pub value: String,
    pub name: String,
    pub type_: String,
    pub n_children: usize,
}

//
// Disassembly stuff
//

/// An assembly instruction. Output by commands like `-data-disassemble`.
#[derive(Debug)]
#[allow(dead_code)]
pub struct AsmInst {
    // TODO: Not sure what this is
    pub offset: usize,
    pub func_name: String,
    pub inst: String,
    pub address: String,
}
