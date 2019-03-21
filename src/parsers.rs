//! Parsing gdb-mi AST (`mi::output_syntax`) to useful types.

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
pub fn parse_frame(v: HashMap<mi::Var, mi::Value>) -> Option<Frame> {
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

pub fn parse_backtrace(v: Vec<(mi::Var, mi::Value)>) -> Option<Backtrace> {
    let mut frames = vec![];
    for (k, v) in v {
        guard!(k == "frame");
        frames.push(parse_frame(v.get_tuple()?)?);
    }
    Some(Backtrace(frames))
}

pub fn parse_breakpoint(v: HashMap<mi::Var, mi::Value>) -> Option<Breakpoint> {
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
    // TODO: what's the difference between "original-location" and "func"? "func" isn't always
    // available
    let original_location = v.get("original-location")?.get_const_ref()?.to_string();
    let file = match v.get("file") {
        None => None,
        Some(file) => Some(file.get_const_ref()?.to_string()),
    };
    let fullname = match v.get("fullname") {
        None => None,
        Some(fullname) => Some(fullname.get_const_ref()?.to_string()),
    };
    let line = match v.get("line") {
        None => None,
        Some(line) => Some(line.get_const_ref()?.parse::<u32>().ok()?),
    };
    // TODO thread-groups
    let cond = match v.get("cond") {
        None => None,
        Some(cond) => Some(cond.get_const_ref()?.to_string()),
    };
    let hits = v.get("times")?.get_const_ref()?.parse::<u32>().ok()?;

    Some(Breakpoint {
        number,
        type_,
        disposition,
        enabled,
        address,
        original_location,
        file,
        fullname,
        line,
        cond,
        hits,
    })
}

pub fn parse_break_insert_result(mut results: HashMap<mi::Var, mi::Value>) -> Option<Breakpoint> {
    parse_breakpoint(results.remove("bkpt")?.get_tuple()?)
}

/// Parse results of a `-var-create` command or a `child` in a `children` list in a
/// `-var-list-children --all-values` result.
fn parse_expr(mut v: HashMap<mi::Var, mi::Value>) -> Option<Value> {
    let expr = match v.remove("exp") {
        None => None,
        Some(expr) => Some(expr.get_const()?),
    };
    let value = v.remove("value")?.get_const()?;
    let name = v.remove("name")?.get_const()?;
    let type_ = v.remove("type")?.get_const()?;
    let n_children = v.remove("numchild")?.get_const()?.parse::<usize>().ok()?;
    Some(Value {
        expr,
        value,
        name,
        type_,
        n_children,
    })
}

pub fn parse_var_create_result(mut results: HashMap<mi::Var, mi::Value>) -> Option<Value> {
    parse_expr(results)
}

pub fn parse_var_list_children_result(
    mut results: HashMap<mi::Var, mi::Value>,
) -> Option<Vec<Value>> {
    println!("parse_var_list_children_result({:?})", results);
    let list = results.remove("children")?.get_result_list()?;
    let mut ret = vec![];
    for (_, child) in list {
        ret.push(parse_expr(child.get_tuple()?)?);
    }
    Some(ret)
}

// >>> -data-disassemble -f <file> -l <line> -n -1 -- 0
// Key: asm_insns, value: list of tuples (input to this function)
pub fn _parse_asm_insts(insts: Vec<mi::Value>) -> Option<Vec<AsmInst>> {
    let mut ret = vec![];
    for inst in insts {
        let mut inst = inst.get_tuple()?;
        let offset = str::parse::<usize>(&inst.remove("offset")?.get_const()?).ok()?;
        let func_name = inst.remove("func-name")?.get_const()?;
        let inst_ = inst.remove("inst")?.get_const()?;
        let address = inst.remove("address")?.get_const()?;
        ret.push(AsmInst {
            offset,
            func_name,
            inst: inst_,
            address,
        });
    }
    Some(ret)
}
