//! Interfacing with gdb via mi. This module only parses gdb messages to mi sytnax
//! (`mi::output_syntax`).

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
    pub fn with_args(mut args0: &[String], mut msg_sender: Sender<mi::Output>) -> GDB {
        let mut args = vec!["-i=mi".to_string(), "--args".to_string()];
        args.extend_from_slice(args0);
        let mut process = Command::new("gdb")
            .args(args.into_iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdout = process.stdout.take().unwrap();
        println!("Spawning gdb-mi message handler");
        let message_handler = thread::spawn(move || message_handler(&mut stdout, &mut msg_sender));

        GDB {
            process,
            message_handler,
        }
    }

    pub fn stdin(&mut self) -> &mut ChildStdin {
        self.process.stdin.as_mut().unwrap()
    }
}

/// mi messages end with this.
/// (Actually they're supposed to end with "(gdb)\r\n" or "(gdb)\n" according to the documentation,
/// but gdb on my system actually terminates messages with "(gdb) \n". Nice.)
static MI_MSG_SEP: &'static str = "(gdb) \n";

/// Read "(gdb)\n" delimited mi messages from `stdout`, send parsed messages to `msg_handler`.
fn message_handler(stdout: &mut ChildStdout, msg_sender: &mut Sender<mi::Output>) {
    // We can't do incremental parsing yet so collect output until we see a "(gdb)\n".
    // We also can't search in a [u8] (nothing like str::find for other slices) so we try to first
    // convert the accumulated output to str.
    let mut msg_bytes = Vec::new();
    loop {
        let mut read_buf: [u8; 10000] = [0; 10000];
        let len = stdout.read(&mut read_buf).unwrap();
        println!("Message handler read {} bytes", len);
        if len == 0 {
            return;
        }
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
        println!("Current message buffer:\n{:?}", msg_str);
        match msg_str.find(MI_MSG_SEP) {
            None => {
                println!("Can't find MI_MSG_SEP");
                continue;
            }
            Some(idx) => {
                let msg = &msg_str[0..idx];
                match mi::parse_output(msg) {
                    None => {
                        println!("Can't parse mi message: {:?}", msg);
                    }
                    Some(mi_msgs) => {
                        // println!("mi message parsed: {:?}", mi_msgs);
                        msg_sender.send(mi_msgs);
                        msg_bytes.drain(0..idx + MI_MSG_SEP.len());
                    }
                }
            }
        }
    }
}
