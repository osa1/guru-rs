// TODO: We need a type for memory locations

//
// Breakpoint stuff
//

#[derive(Debug)]
pub struct Breakpoint {
    pub number: usize,

    pub type_: BreakpointType,

    /// Should the breakpoint be deleted or disabled when it is hit?
    pub disposition: BreakpointDisposition,

    pub enabled: bool,

    /// Memory location at which the breakpoint is set.
    pub address: String,

    /// Logical location of the breakpoint, expressed by function name, file name, line number.
    pub what: String,

    // TODO thread-groups?

    /// Number of times the breakpoint has been hit
    pub times: usize,
}

// NOTE: GDB has more details like whether the watchpoint is hardware or not. We ignore those for
// now.
#[derive(Debug, PartialEq, Eq)]
pub enum BreakpointType {
    Breakpoint,
    Watchpoint,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BreakpointDisposition {
    Keep,
    NoKeep,
}

//
// Backtrace stuff
//

pub struct Backtrace(pub Vec<Frame>);

pub struct Frame {
    pub level: usize,

    /// The $pc value for the frame.
    pub addr: String,

    /// Function name
    pub func: String,

    /// File name of the source file where the function lives.
    pub file: String,

    /// The full file name of the source file where the function lives.
    pub fullname: String,

    /// Line number corresponding to the $pc.
    pub line: usize,

    /// The shared library where this function is defined. This is only given if the frame’s
    /// function is not known.
    pub from: Option<String>,
}
