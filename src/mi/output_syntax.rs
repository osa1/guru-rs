pub use super::syntax_common::*;

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Const(String),
    Tuple(HashMap<Variable, Value>),
    ValueList(Vec<Value>),
    ResultList(Vec<(Variable, Value)>),
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
