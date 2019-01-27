pub use super::syntax_common::*;

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Const(String),
    Tuple(HashMap<Variable, Value>),
    ValueList(Vec<Value>),
    ResultList(Vec<(Variable, Value)>),
}

impl Value {
    pub fn get_const_ref(&self) -> Option<&str> {
        match self {
            Value::Const(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn get_tuple(self) -> Option<HashMap<Variable, Value>> {
        match self {
            Value::Tuple(m) => Some(m),
            _ => None,
        }
    }

    pub fn get_result_list(self) -> Option<Vec<(Variable, Value)>> {
        match self {
            Value::ResultList(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Output {
    pub out_of_band: Vec<OutOfBandResult>,
    pub result: Option<Result>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Result {
    pub token: Option<Token>,
    pub class: ResultClass,
    pub results: Vec<(Variable, Value)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResultClass {
    Done,
    Running,
    Connected,
    Error,
    Exit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutOfBandResult {
    ExecAsyncRecord(AsyncRecord),
    StatusAsyncRecord(AsyncRecord),
    NotifyAsyncRecord(AsyncRecord),
    ConsoleStreamRecord(String),
    TargetStreamRecord(String),
    LogStreamRecord(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct AsyncRecord {
    pub token: Option<Token>,
    pub class: String,
    pub results: Vec<(Variable, Value)>,
}
