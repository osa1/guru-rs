//! Interfacing with gdb via mi.

use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str;
use std::thread;

use glib::Sender;

use crate::mi;

pub struct GDB {
    process: Child,
    message_handler: thread::JoinHandle<()>,
}

impl GDB {
    /// Spawn a new GDB process with the given args. The args will be passed to gdb like this
    /// ```
    /// $ gdb --args <args>
    /// ```
    /// A spawns that reads gdb stdout and sends parsed mi messages to `msg_sender` will be
    /// spawned.
    pub fn with_args(mut args: Vec<String>, mut msg_sender: Sender<mi::Output>) -> GDB {
        args.insert(0, "--args".to_string());
        let mut process = Command::new("gdb")
            .args(args.into_iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdout = process.stdout.take().unwrap();
        let message_handler = thread::spawn(move || message_handler(&mut stdout, &mut msg_sender));

        GDB {
            process,
            message_handler,
        }
    }

    /// Get stdin of the spawned gdb process. Note that this moves the stdin so this returns `None`
    /// after the first call.
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.process.stdin.take()
    }
}

/// mi messages end with this.
static MI_MSG_SEP: &'static str = "(gdb)\n";

/// Read "(gdb)\n" delimited mi messages from `stdout`, send parsed messages to `msg_handler`.
fn message_handler(stdout: &mut ChildStdout, msg_sender: &mut Sender<mi::Output>) {
    // We can't do incremental parsing yet so collect output until we see a "(gdb)\n".
    // We also can't search in a [u8] (nothing like str::find for other slices) so we try to first
    // convert the accumulated output to str.
    let mut msg_bytes = Vec::new();
    loop {
        let mut read_buf: [u8; 10000] = [0; 10000];
        let len = stdout.read(&mut read_buf).unwrap();
        msg_bytes.extend_from_slice(&read_buf[0..len]);
        let msg_str = match str::from_utf8(&msg_bytes) {
            Err(err) => {
                // sigh so many hacks ...
                println!(
                    "mi message is not valid utf-8! {}\n\
                     msg: {:?}",
                    err, msg_bytes
                );
                continue;
            }
            Ok(str) => str,
        };
        match msg_str.find(MI_MSG_SEP) {
            None => {
                continue;
            }
            Some(idx) => {
                let idx = idx + MI_MSG_SEP.len();
                let msg = &msg_str[0..idx];
                match mi::parse_output(msg) {
                    None => {
                        println!("Can't parse mi message: {}", msg);
                    }
                    Some((mi_msg, rest)) => {
                        assert!(rest.is_empty());
                        msg_sender.send(mi_msg);
                        msg_bytes.drain(0..idx);
                    }
                }
            }
        }
    }
}
