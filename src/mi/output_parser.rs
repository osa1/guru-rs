//! Parsing GDB-mi outputs.

use super::output_syntax::*;

macro_rules! guard {
    ( $x:expr ) => {
        if !$x {
            return None;
        }
    };
}

// All parsers return result + unconsumed input.

// [26/01/2019] This code makes me want to kill myself

pub fn parse_output(s: &str) -> Option<(Output, &str)> {
    let (out_of_band, mut s) = parse_out_of_bands(s);
    let result = match parse_result_record(s) {
        None => None,
        Some((result, s_)) => {
            s = s_;
            Some(result)
        }
    };
    guard!(s == "(gdb)\n");
    Some((
        Output {
            out_of_band,
            result,
        },
        &s["(gdb)\n".len()..],
    ))
}

fn parse_out_of_bands(mut s: &str) -> (Vec<OutOfBandResult>, &str) {
    let mut ret = vec![];

    loop {
        match parse_out_of_band(s) {
            Some((out_of_band, s_)) => {
                s = s_;
                ret.push(out_of_band);
            }
            None => {
                break;
            }
        }
    }

    (ret, s)
}

fn parse_out_of_band(s: &str) -> Option<(OutOfBandResult, &str)> {
    // TODO: Would be good to reduce duplication below
    match parse_token(s) {
        None => {
            // async-record or a stream-record
            let c = s.chars().next()?;
            let s = &s[c.len_utf8()..];
            match c {
                '*' => {
                    let (async_record, s) = parse_async_record(s)?;
                    Some((OutOfBandResult::ExecAsyncRecord(async_record), s))
                }
                '+' => {
                    let (async_record, s) = parse_async_record(s)?;
                    Some((OutOfBandResult::StatusAsyncRecord(async_record), s))
                }
                '=' => {
                    let (async_record, s) = parse_async_record(s)?;
                    Some((OutOfBandResult::NotifyAsyncRecord(async_record), s))
                }
                '~' => {
                    let (stream_record, s) = parse_stream_record(s)?;
                    Some((OutOfBandResult::ConsoleStreamRecord(stream_record), s))
                }
                '@' => {
                    let (stream_record, s) = parse_stream_record(s)?;
                    Some((OutOfBandResult::TargetStreamRecord(stream_record), s))
                }
                '&' => {
                    let (stream_record, s) = parse_stream_record(s)?;
                    Some((OutOfBandResult::LogStreamRecord(stream_record), s))
                }
                _ => None,
            }
        }
        Some((token, s)) => {
            // stream-record doesn't have token so this has to be an async-record
            let c = s.chars().next()?;
            let s = &s[c.len_utf8()..];
            match c {
                '*' => {
                    let (mut async_record, s) = parse_async_record(s)?;
                    async_record.token = Some(token);
                    Some((OutOfBandResult::ExecAsyncRecord(async_record), s))
                }
                '+' => {
                    let (mut async_record, s) = parse_async_record(s)?;
                    async_record.token = Some(token);
                    Some((OutOfBandResult::StatusAsyncRecord(async_record), s))
                }
                '=' => {
                    let (mut async_record, s) = parse_async_record(s)?;
                    async_record.token = Some(token);
                    Some((OutOfBandResult::NotifyAsyncRecord(async_record), s))
                }
                _ => None,
            }
        }
    }
}

// result-record → [ token ] "^" result-class ( "," result )* nl
fn parse_result_record(mut s: &str) -> Option<(Result, &str)> {
    let token = {
        match parse_token(s) {
            None => None,
            Some((token, s_)) => {
                s = s_;
                Some(token)
            }
        }
    };
    guard!(s.chars().next()? == '^');
    let class = if s.starts_with("done") {
        s = &s["done".len()..];
        ResultClass::Done
    } else if s.starts_with("running") {
        s = &s["running".len()..];
        ResultClass::Running
    } else if s.starts_with("connected") {
        s = &s["connected".len()..];
        ResultClass::Connected
    } else if s.starts_with("error") {
        s = &s["error".len()..];
        ResultClass::Error
    } else if s.starts_with("exit") {
        s = &s["exit".len()..];
        ResultClass::Exit
    } else {
        return None;
    };

    let mut results = vec![];
    loop {
        let c = s.chars().next()?;
        if c == ',' {
            let (result, s_) = parse_result(s)?;
            results.push(result);
            s = s_;
        } else if c == '\n' {
            return Some((
                Result {
                    token,
                    class,
                    results,
                },
                s,
            ));
        } else {
            return None;
        }
    }
}

fn parse_token(s: &str) -> Option<(Token, &str)> {
    guard!(!s.is_empty());
    let mut token = String::new();
    let mut c_idx = 0;
    for c in s.chars() {
        if c >= '0' && c <= '9' {
            token.push(c);
            c_idx += c.len_utf8();
        } else {
            if token.is_empty() {
                return None;
            }
            break;
        }
    }
    Some((token, &s[c_idx..]))
}

fn parse_async_record(mut s: &str) -> Option<(AsyncRecord, &str)> {
    let class = {
        let mut class = String::new();
        let mut cs = s.chars();
        loop {
            let c = cs.next()?;
            if c == '\n' {
                let class_len = class.len();
                return Some((
                    AsyncRecord {
                        token: None,
                        class: class,
                        results: vec![],
                    },
                    &s[class_len + '\n'.len_utf8()..],
                ));
            } else if c == ',' {
                // Dont' skip ',' here!
                s = &s[class.len()..];
                break;
            } else {
                class.push(c);
            }
        }
        class
    };
    let mut results = vec![];
    while s.chars().next() == Some(',') {
        let (result, s_) = parse_result(&s[','.len_utf8()..])?;
        s = s_;
        results.push(result);
    }
    guard!(s.chars().next()? == '\n');
    Some((
        AsyncRecord {
            token: None,
            class,
            results,
        },
        &s['\n'.len_utf8()..],
    ))
}

// result → variable "=" value
fn parse_result(s: &str) -> Option<((Variable, Value), &str)> {
    let (var, mut s) = parse_variable(s)?;
    guard!(s.chars().next()? == '=');
    s = &s['='.len_utf8()..];
    let (val, s) = parse_value(s)?;
    Some(((var, val), s))
}

// variable → string
// It's not clear what a string is though.
fn parse_variable(s: &str) -> Option<(Variable, &str)> {
    let mut ret = String::new();
    let mut c_idx = 0;
    for c in s.chars() {
        if c != '=' && c != ',' && !c.is_whitespace() {
            ret.push(c);
            c_idx += c.len_utf8();
        } else {
            if ret.is_empty() {
                return None;
            } else {
                break;
            }
        }
    }
    Some((ret, &s[c_idx..]))
}

// value → const | tuple | list
// const → c-string
// tuple → "{}" | "{" result ( "," result )* "}"
// list  → "[]" | "[" value ( "," value )* "]" | "[" result ( "," result )* "]"
fn parse_value(s: &str) -> Option<(Value, &str)> {
    let c = s.chars().next()?;
    let s = &s[c.len_utf8()..];
    match c {
        '"' => {
            let mut ret = String::new();
            let mut c_idx = 0;
            for c in s.chars() {
                c_idx += c.len_utf8();
                if c == '"' {
                    return Some((Value::Const(ret), &s[c_idx..]));
                } else {
                    ret.push(c);
                }
            }
            None
        }
        '{' => {
            let mut tuple = vec![];
            let mut s = s;
            loop {
                match parse_result(s) {
                    None => {
                        if s.chars().next()? == '}' {
                            return Some((Value::Tuple(tuple), &s['}'.len_utf8()..]));
                        } else {
                            return None;
                        }
                    }
                    Some((result, s_)) => {
                        tuple.push(result);
                        s = s_;
                        let c = s_.chars().next()?;
                        // This allows more than we need but whatever
                        if c == '}' {
                            return Some((Value::Tuple(tuple), &s['}'.len_utf8()..]));
                        } else if c == ',' {
                            continue;
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        '[' => {
            // Value or result list?
            if s.chars().next()? == ']' {
                return Some((Value::ValueList(vec![]), &s[']'.len_utf8()..]));
            }
            match parse_value(s) {
                None => {
                    // Should be a result list
                    let mut results = vec![];
                    let (result0, s) = parse_result(s)?;
                    results.push(result0);
                    let mut s = s;
                    loop {
                        let c = s.chars().next()?;
                        if c == ',' {
                            let (result, s_) = parse_result(s)?;
                            results.push(result);
                            s = s_;
                        } else if c == '}' {
                            return Some((Value::ResultList(results), &s[c.len_utf8()..]));
                        } else {
                            return None;
                        }
                    }
                }
                Some((value0, s)) => {
                    // Value list
                    let mut values = vec![value0];
                    let (value, s) = parse_value(s)?;
                    values.push(value);
                    let mut s = s;
                    loop {
                        let c = s.chars().next()?;
                        if c == ',' {
                            let (value, s_) = parse_value(s)?;
                            values.push(value);
                            s = s_;
                        } else if c == '}' {
                            return Some((Value::ValueList(values), &s[c.len_utf8()..]));
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        _ => None,
    }
}

// stream-record         → console-stream-output | target-stream-output | log-stream-output
// console-stream-output → "~" c-string nl
// target-stream-output  → "@" c-string nl
// log-stream-output     → "&" c-string nl
//
// c-string is dquote delimited anything.
//
// Note that we don't parse ~/@/& here. Those are parsed at the call site.
fn parse_stream_record(mut s: &str) -> Option<(String, &str)> {
    guard!(s.chars().next()? == '"');
    s = &s['"'.len_utf8()..];
    let mut output = String::new();
    let mut c_idx = 0;
    for c in s.chars() {
        c_idx += c.len_utf8();
        if c == '"' {
            break;
        } else {
            output.push(c);
        }
    }
    s = &s[c_idx..];
    guard!(s.chars().next()? == '\n');
    Some((output, &s['\n'.len_utf8()..]))
}

#[test]
fn parse_token_tests() {
    assert_eq!(parse_token(""), None);
    assert_eq!(parse_token("123*"), Some(("123".to_string(), "*")));
    assert_eq!(parse_token("*"), None);
}

#[test]
fn parse_variable_tests() {
    assert_eq!(
        parse_variable("param=\"foo\""),
        Some(("param".to_string(), "=\"foo\""))
    );
}

#[test]
fn parse_result_tests() {
    assert_eq!(
        parse_result("value=\"on\""),
        Some((("value".to_string(), Value::Const("on".to_string())), ""))
    );
}

#[test]
fn parse_value_tests() {
    assert_eq!(
        parse_value("\"foo\""),
        Some((Value::Const("foo".to_string()), ""))
    );
    assert_eq!(parse_value("{}"), Some((Value::Tuple(vec![]), "")));
    assert_eq!(parse_value("[]"), Some((Value::ValueList(vec![]), "")));
}

#[test]
fn parse_out_of_band_tests() {
    assert_eq!(
        parse_out_of_band("=thread-group-added\n"),
        Some((
            OutOfBandResult::NotifyAsyncRecord(AsyncRecord {
                token: None,
                class: "thread-group-added".to_string(),
                results: vec![]
            }),
            ""
        ))
    );
    assert_eq!(
        parse_out_of_band("=thread-group-added,id=\"i1\"\n"),
        Some((
            OutOfBandResult::NotifyAsyncRecord(AsyncRecord {
                token: None,
                class: "thread-group-added".to_string(),
                results: vec![("id".to_string(), Value::Const("i1".to_string()))]
            }),
            ""
        ))
    );
    assert_eq!(
        parse_out_of_band("*running,thread-id=\"5\"\n"),
        Some(
            (OutOfBandResult::ExecAsyncRecord(AsyncRecord {
                token: None,
                class: "running".to_string(),
                results: vec![("thread-id".to_string(), Value::Const("5".to_string()))]
            }), "")
        )
    )
}

#[test]
fn parse_output_tests() {
    let out = Output {
        out_of_band: vec![OutOfBandResult::NotifyAsyncRecord(AsyncRecord {
            token: None,
            class: "thread-group-added".to_string(),
            results: vec![("id".to_string(), Value::Const("i1".to_string()))],
        })],
        result: None,
    };
    assert_eq!(
        parse_output("=thread-group-added,id=\"i1\"\n(gdb)\n"),
        Some((out, ""))
    );

    let out = Output {
        out_of_band: vec![OutOfBandResult::NotifyAsyncRecord(AsyncRecord {
            token: None,
            class: "cmd-param-changed".to_string(),
            results: vec![
                (
                    "param".to_string(),
                    Value::Const("history save".to_string()),
                ),
                ("value".to_string(), Value::Const("on".to_string())),
            ],
        })],
        result: None,
    };
    assert_eq!(
        parse_output("=cmd-param-changed,param=\"history save\",value=\"on\"\n(gdb)\n"),
        Some((out, ""))
    );

    let s = "=thread-group-added,id=\"i1\"\n\
             =cmd-param-changed,param=\"history save\",value=\"on\"\n\
             =cmd-param-changed,param=\"confirm\",value=\"off\"\n\
             =cmd-param-changed,param=\"print pretty\",value=\"on\"\n\
             =cmd-param-changed,param=\"print array-indexes\",value=\"on\"\n\
             =cmd-param-changed,param=\"python print-stack\",value=\"full\"\n\
             =cmd-param-changed,param=\"pagination\",value=\"off\"\n\
             (gdb)\n";
    assert_eq!(parse_output(s).map(|t| t.1), Some(""));

    let s = "~\"Reading symbols from gc_test...\"\n\
             (gdb)\n";
    let out = Output {
        out_of_band: vec![OutOfBandResult::ConsoleStreamRecord(
            "Reading symbols from gc_test...".to_string(),
        )],
        result: None,
    };
    assert_eq!(parse_output(s), Some((out, "")));
}
