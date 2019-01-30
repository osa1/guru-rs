//! Parsing GDB-mi output to the AST defined in `mi::output_syntax`.

// https://sourceware.org/gdb/onlinedocs/gdb/GDB_002fMI-Output-Syntax.html#GDB_002fMI-Output-Syntax

use super::output_syntax::*;

use std::collections::HashMap;

macro_rules! guard {
    ( $x:expr ) => {
        if !$x {
            return None;
        }
    };
}

// All parsers return result + unconsumed input, except `parse_output` becuase it tries to parse
// the whole input.

// [26/01/2019] This code makes me want to kill myself

// [29/01/2019] gdb-mi documentation is wrong about the syntax: a single message may contain
// out-of-band records, then a result, then more out-of-band records. It's also wrong about the
// message terminator ("(gdb)\n" etc.). So we don't expect to see a terminator here, and we parse
// any ordering of out-of-band results and normal results (e.g. we accept [oob, normal, oob,
// normal] etc.).

// [30/01/2019] Here's another bug with gdb mi: when a breakpoint location causes adding multiple
// breakpoints the notification is printed like this:
//
//      =breakpoint-created,bkpt={...},{...},{...}
//
// But this is not valid async record syntax. The results should be in `x=y` format so the
// breakpoints after the first one (those without `bkpt=`) are not valid.
//
// I can't even build gdb on Ubuntu 18.04, and even if I could asking every user to install gdb
// HEAD would be asking too much. So to deal with this we extend make the AST more flexible, and
// make the LHS (`bkpt=` part) optional. *sigh*

pub fn parse_output(mut s: &str) -> Option<Output> {
    let mut ret = vec![];

    while !s.is_empty() {
        match parse_out_of_band(s) {
            None => match parse_result_record(s) {
                None => {
                    return None;
                }
                Some((res, s_)) => {
                    ret.push(ResultOrOOB::Result(res));
                    s = s_;
                }
            },
            Some((oob, s_)) => {
                ret.push(ResultOrOOB::OOB(oob));
                s = s_;
            }
        }
    }

    Some(ret)
}

// Expect a newline, then consume any subsequent newlines. According to gdb manual only one newline
// should be between OOBs/results, but in practice I've seen more than one newlines between
// them.
fn expect_newline(s: &str) -> Option<&str> {
    guard!(s.chars().next()? == '\n');
    for (c_idx, c) in s.char_indices() {
        if c != '\n' {
            return Some(&s[c_idx..]);
        }
    }
    Some("")
}

// out-of-band-record → async-record | stream-record
// async-record → exec-async-output | status-async-output | notify-async-output
// exec-async-output → [ token ] "*" async-output nl
// status-async-output → [ token ] "+" async-output nl
// notify-async-output → [ token ] "=" async-output nl
// stream-record → console-stream-output | target-stream-output | log-stream-output
// console-stream-output → "~" c-string nl
// target-stream-output → "@" c-string nl
// log-stream-output → "&" c-string nl
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
                    let (stream_record, s) = parse_string(s)?;
                    let s = expect_newline(s)?;
                    Some((OutOfBandResult::ConsoleStreamRecord(stream_record), s))
                }
                '@' => {
                    let (stream_record, s) = parse_string(s)?;
                    let s = expect_newline(s)?;
                    Some((OutOfBandResult::TargetStreamRecord(stream_record), s))
                }
                '&' => {
                    let (stream_record, s) = parse_string(s)?;
                    let s = expect_newline(s)?;
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
    s = &s['^'.len_utf8()..];
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

    let mut results = HashMap::new();
    loop {
        let c = s.chars().next()?;
        if c == ',' {
            s = &s[c.len_utf8()..];
            let ((var, val), s_) = parse_result(s)?;
            assert!(!results.contains_key(&var));
            results.insert(var, val);
            s = s_;
        } else if c == '\n' {
            let s = expect_newline(s)?;
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
                let s = expect_newline(&s[class_len..])?;
                return Some((
                    AsyncRecord {
                        token: None,
                        class: class,
                        results: HashMap::new(),
                    },
                    s,
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
    let mut results = HashMap::new();
    while s.chars().next() == Some(',') {
        let ((var, val), s_) = parse_result(&s[','.len_utf8()..])?;
        s = s_;
        assert!(!results.contains_key(&var));
        results.insert(var, val);
    }
    let s = expect_newline(s)?;
    Some((
        AsyncRecord {
            token: None,
            class,
            results,
        },
        s,
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
pub fn parse_value(s: &str) -> Option<(Value, &str)> {
    let c = s.chars().next()?;
    match c {
        '"' => parse_string(s).map(|(ret, s)| (Value::Const(ret), s)),
        '{' => {
            let s = &s[c.len_utf8()..];
            let mut tuple = HashMap::new();
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
                    Some(((k, v), s_)) => {
                        assert!(!tuple.contains_key(&k));
                        tuple.insert(k, v);
                        s = s_;
                        let c = s.chars().next()?;
                        // This allows more than we need but whatever
                        if c == '}' {
                            return Some((Value::Tuple(tuple), &s[c.len_utf8()..]));
                        } else if c == ',' {
                            s = &s[c.len_utf8()..];
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
            let s = &s[c.len_utf8()..];
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
                            let (result, s_) = parse_result(&s[c.len_utf8()..])?;
                            results.push(result);
                            s = s_;
                        } else if c == ']' {
                            return Some((Value::ResultList(results), &s[c.len_utf8()..]));
                        } else {
                            return None;
                        }
                    }
                }
                Some((value0, s)) => {
                    // Value list
                    let mut values = vec![value0];
                    let mut s = s;
                    loop {
                        let c = s.chars().next()?;
                        if c == ',' {
                            let (value, s_) = parse_value(&s[c.len_utf8()..])?;
                            values.push(value);
                            s = s_;
                        } else if c == ']' {
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

fn parse_string(mut s: &str) -> Option<(String, &str)> {
    guard!(s.chars().next()? == '"');
    s = &s['"'.len_utf8()..];
    let mut output = String::new();
    let mut c_idx = 0;
    let mut escape = false;
    for c in s.chars() {
        c_idx += c.len_utf8();
        if escape {
            if c == '\\' {
                output.push(c);
            } else if c == 'n' {
                output.push('\n');
            } else if c == '"' {
                output.push('"');
            } else if c == 't' {
                output.push('\t');
            } else {
                println!("Unknown escape character: {}", c);
                output.push(c);
            }
            escape = false;
        } else {
            if c == '\\' {
                escape = true;
            } else if c == '"' {
                break;
            } else {
                output.push(c);
            }
        }
    }
    s = &s[c_idx..];
    Some((output, s))
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
    assert_eq!(parse_value("{}"), Some((Value::Tuple(HashMap::new()), "")));
    assert_eq!(parse_value("[]"), Some((Value::ValueList(vec![]), "")));

    let input = "[frame={level=\"0\",addr=\"0x00000000006eff82\",func=\"initCapabilities\",file=\
                 \"rts/Capability.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/Capability.c\",\
                 line=\"398\"},frame={level=\"1\",addr=\"0x00000000006ee476\",func=\"initScheduler\
                 \",file=\"rts/Schedule.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/Schedule.c\
                 \",line=\"2680\"},frame={level=\"2\",addr=\"0x00000000006e8cc0\",\
                 func=\"hs_init_ghc\",file=\"rts/RtsStartup.c\",fullname=\
                 \"/home/omer/haskell/ghc-gc/rts/RtsStartup.c\",line=\"236\"},frame={level=\"3\"\
                 ,addr=\"0x0000000000701f08\",func=\"hs_main\",file=\"rts/RtsMain.c\",\
                 fullname=\"/home/omer/haskell/ghc-gc/rts/RtsMain.c\",line=\"57\"},\
                 frame={level=\"4\",addr=\"0x0000000000405366\",func=\"main\"}]";
    let out = parse_value(input);
    assert!(out.is_some());
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
        Some((
            OutOfBandResult::ExecAsyncRecord(AsyncRecord {
                token: None,
                class: "running".to_string(),
                results: vec![("thread-id".to_string(), Value::Const("5".to_string()))]
            }),
            ""
        ))
    )
}

#[test]
fn parse_output_tests() {
    let out = vec![ResultOrOOB::OOB(OutOfBandResult::NotifyAsyncRecord(
        AsyncRecord {
            token: None,
            class: "thread-group-added".to_string(),
            results: vec![("id".to_string(), Value::Const("i1".to_string()))],
        },
    ))];
    assert_eq!(parse_output("=thread-group-added,id=\"i1\"\n"), Some(out));

    let out = vec![ResultOrOOB::OOB(OutOfBandResult::NotifyAsyncRecord(
        AsyncRecord {
            token: None,
            class: "cmd-param-changed".to_string(),
            results: vec![
                (
                    "param".to_string(),
                    Value::Const("history save".to_string()),
                ),
                ("value".to_string(), Value::Const("on".to_string())),
            ],
        },
    ))];
    assert_eq!(
        parse_output("=cmd-param-changed,param=\"history save\",value=\"on\"\n"),
        Some(out)
    );

    let s = "=thread-group-added,id=\"i1\"\n\
             =cmd-param-changed,param=\"history save\",value=\"on\"\n\
             =cmd-param-changed,param=\"confirm\",value=\"off\"\n\
             =cmd-param-changed,param=\"print pretty\",value=\"on\"\n\
             =cmd-param-changed,param=\"print array-indexes\",value=\"on\"\n\
             =cmd-param-changed,param=\"python print-stack\",value=\"full\"\n\
             =cmd-param-changed,param=\"pagination\",value=\"off\"\n";
    assert_eq!(parse_output(s).map(|r| r.len()), Some(7));

    let s = "~\"Reading symbols from gc_test...\"\n";
    let out = vec![ResultOrOOB::OOB(OutOfBandResult::ConsoleStreamRecord(
        "Reading symbols from gc_test...".to_string(),
    ))];
    assert_eq!(parse_output(s), Some(out));

    let s = "~\"\\\"\"\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(1));

    let s = "^done\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(1));

    let s = "^error,msg=\"Undefined command: \\\"halp\\\".  Try \\\"help\\\".\"\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(1));

    let s = "^running\n*running,thread-id=\"all\"\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(2));

    let s = "*stopped,frame={args=[{name=\"cap\",value=\"0x4de0c0 <MainCapability>\"},{name=\"idle_cap\",value=\"0x507670\"}]}\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(1));

    let s = "~\"[Thread debugging using libthread_db enabled]\\n\"\n\
             *running,thread-id=\"3\"\n\
             =thread-created,id=\"4\",group-id=\"i1\"\n\
             *running,thread-id=\"4\"\n\
             =thread-created,id=\"5\",group-id=\"i1\"\n\
             ~\"[New Thread 0x7fffef7fe700 (LWP 4044)]\\n\"\n\
             *running,thread-id=\"5\"\n\n\n\n\
             ~\"[Thread 0x7fffef7fe700 (LWP 4044) exited]\\n\"\n\
             =thread-exited,id=\"5\",group-id=\"i1\"\n\
             ~\"[Thread 0x7fffeffff700 (LWP 4043) exited]\\n\"\n\
             =thread-exited,id=\"4\",group-id=\"i1\"\n\
             ~\"[Thread 0x7ffff4d2f700 (LWP 4042) exited]\\n\"\n\
             =thread-exited,id=\"3\",group-id=\"i1\"\n\
             ~\"[Thread 0x7ffff5530700 (LWP 4041) exited]\\n\"\n\
             =thread-exited,id=\"2\",group-id=\"i1\"\n\
             ~\"[Inferior 1 (process 4037) exited normally]\\n\"\n\
             =thread-exited,id=\"1\",group-id=\"i1\"\n\
             =thread-group-exited,id=\"i1\",exit-code=\"0\"\n\
             *stopped,reason=\"exited-normally\"\n";
    assert_eq!(parse_output(s).map(|t| t.len()), Some(19));
}
