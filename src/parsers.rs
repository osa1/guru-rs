/// Parsing gdb-mi AST (`mi::output_syntax`) to useful types.
use crate::mi::output_syntax as mi;
use crate::types::*;

use std::collections::HashMap;

// TODO: copied from mi::parser
macro_rules! guard {
    ( $x:expr ) => {
        if !$x {
            return None;
        }
    };
}

/// Parse a single frame.
pub fn parse_frame(v: HashMap<mi::Variable, mi::Value>) -> Option<Frame> {
    println!("parse frame: {:?}", v);
    Some(Frame {
        level: v.get("level")?.get_const_ref()?.parse::<usize>().ok()?,
        addr: v.get("addr")?.get_const_ref()?.to_string(),
        func: v.get("func")?.get_const_ref()?.to_string(),
        file: match v.get("file") {
            None => None,
            Some(file) => Some(file.get_const_ref()?.to_string()),
        },
        fullname: match v.get("fullname") {
            None => None,
            Some(fullname) => Some(fullname.get_const_ref()?.to_string()),
        },
        line: match v.get("line") {
            None => None,
            Some(line) => Some(line.get_const_ref()?.parse::<usize>().ok()?),
        },
        from: None, // TODO
    })
}

pub fn parse_backtrace(v: Vec<(mi::Variable, mi::Value)>) -> Option<Backtrace> {
    let mut frames = vec![];
    for (k, v) in v {
        guard!(k == "frame");
        frames.push(parse_frame(v.get_tuple()?)?);
    }
    Some(Backtrace(frames))
}

pub fn parse_breakpoint(v: HashMap<mi::Variable, mi::Value>) -> Option<Breakpoint> {
    let number = v.get("number")?.get_const_ref()?.parse::<u32>().ok()?;
    let type_ = {
        guard!(v.get("type")?.get_const_ref()? == "breakpoint");
        BreakpointType::Breakpoint
    };
    let disposition = match v.get("disp")?.get_const_ref()? {
        "keep" => BreakpointDisposition::Keep,
        "nokeep" => BreakpointDisposition::NoKeep,
        _ => {
            return None;
        }
    };
    let enabled = match v.get("enabled")?.get_const_ref()? {
        "y" => true,
        "n" => false,
        _ => {
            return None;
        }
    };
    let address = v.get("addr")?.get_const_ref()?.to_string();
    let func = v.get("func")?.get_const_ref()?.to_string();
    let file = v.get("file")?.get_const_ref()?.to_string();
    let fullname = v.get("fullname")?.get_const_ref()?.to_string();
    let line = v.get("line")?.get_const_ref()?.parse::<u32>().ok()?;
    // TODO thread-groups
    let cond = match v.get("cond") {
        None => None,
        Some(cond) => Some(cond.get_const_ref()?.to_string()),
    };
    let hits = v.get("times")?.get_const_ref()?.parse::<u32>().ok()?;
    // TODO original-location

    Some(Breakpoint {
        number,
        type_,
        disposition,
        enabled,
        address,
        func,
        file,
        fullname,
        line,
        cond,
        hits,
    })
}
