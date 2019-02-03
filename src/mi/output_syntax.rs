use std::collections::HashMap;

pub type Var = String;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Const(String),
    Tuple(HashMap<Var, Value>),
    ValueList(Vec<Value>),
    // NOTE: Do not make this a HashMap<Var, Value>! A result list may contain same variable
    // multiple times.
    ResultList(Vec<(Var, Value)>),
}

impl Value {
    pub fn get_const_ref(&self) -> Option<&str> {
        match self {
            Value::Const(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn get_const(self) -> Option<String> {
        match self {
            Value::Const(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_tuple(self) -> Option<HashMap<Var, Value>> {
        match self {
            Value::Tuple(m) => Some(m),
            _ => None,
        }
    }

    pub fn get_result_list(self) -> Option<Vec<(Var, Value)>> {
        match self {
            Value::ResultList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_value_list(self) -> Option<Vec<Value>> {
        match self {
            Value::ValueList(values) => Some(values),
            _ => None,
        }
    }
}

// This is different than the syntax defined in gdb-mi documentation because the documentation is
// wrong. See comments around `mi::parser::parse_output`.
pub type Output = Vec<ResultOrOOB>;

#[derive(Debug, PartialEq, Eq)]
pub enum ResultOrOOB {
    Result(Result),
    OOB(OutOfBandResult),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Result {
    pub token: Option<u64>,
    pub class: ResultClass,
    pub results: HashMap<Var, Value>,
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
    pub token: Option<u64>,
    pub class: String,
    pub results: HashMap<Var, Value>,
}
