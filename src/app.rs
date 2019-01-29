use crate::gdb;
use crate::mi;
use crate::widgets;

use gio::prelude::*;
use gtk::prelude::*;

pub struct App {
    // Widgets
    threads_w: widgets::ThreadsW,
    breakpoints_w: widgets::BreakpointsW,
    gdb_w: widgets::GdbW,
    // GDB driver
    gdb: Option<gdb::GDB>,
}

impl App {
    pub fn new(gtk_app: &gtk::Application) -> App {
        let window = gtk::ApplicationWindow::new(gtk_app);
        window.set_default_size(500, 850);
        window.set_title("guru");

        // Horizontal: | Vertical: -

        // Current layout:
        // horiz(1) ->
        //   [ vert(1) -> [ vert(2) -> [ currently_empty, gdb logs ], breakpoints ]
        //   , threads
        //   ]

        let horiz1 = gtk::Paned::new(gtk::Orientation::Horizontal);
        window.add(&horiz1);

        let vert1 = gtk::Paned::new(gtk::Orientation::Vertical);
        horiz1.pack1(&vert1, true, false);

        let vert2 = gtk::Paned::new(gtk::Orientation::Vertical);
        vert1.pack1(&vert2, true, false);

        let gdb_w = widgets::GdbW::new();
        vert2.pack2(gdb_w.get_widget(), true, false);

        let breakpoints_w = widgets::BreakpointsW::new();
        vert1.pack2(breakpoints_w.get_widget(), true, false);

        let threads_w = widgets::ThreadsW::new();
        horiz1.pack2(threads_w.get_widget(), true, false);

        window.show_all();

        App {
            threads_w,
            breakpoints_w,
            gdb_w,
            gdb: None,
        }
    }

    pub fn set_gdb(&mut self, gdb: gdb::GDB) {
        self.gdb = Some(gdb);
    }

    pub fn mi_msg_recvd(&mut self, mi_msg: mi::Output) -> gtk::Continue {
        for oob in mi_msg.out_of_band {
            match oob {
                mi::OutOfBandResult::ExecAsyncRecord(async_) => {
                    println!("Adding exec async record: {:?}", async_);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#505B70\">[EXEC]</span> {}",
                        render_async_record(async_)
                    ));
                }
                mi::OutOfBandResult::StatusAsyncRecord(async_) => {
                    println!("Adding status async record: {:?}", async_);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#3FBCA6\">[STATUS]</span> {}",
                        render_async_record(async_)
                    ));
                }
                mi::OutOfBandResult::NotifyAsyncRecord(async_) => {
                    println!("Adding notify async record: {:?}", async_);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#CBCE79\">[NOTIFY]</span> {}",
                        render_async_record(async_)
                    ));
                }
                mi::OutOfBandResult::ConsoleStreamRecord(str) => {
                    println!("Adding console stream record: {:?}", str);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#A1D490\">[CONSOLE]</span> {}",
                        escape_brackets(&str)
                    ));
                }
                mi::OutOfBandResult::TargetStreamRecord(str) => {
                    println!("Adding target stream record: {:?}", str);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#90C3D4\">[TARGET]</span> {}",
                        escape_brackets(&str)
                    ));
                }
                mi::OutOfBandResult::LogStreamRecord(str) => {
                    println!("Adding log stream record: {:?}", str);
                    self.gdb_w.insert_line(&format!(
                        "<span color=\"#D4A190\">[LOG]</span> {}",
                        escape_brackets(&str)
                    ));
                }
            }
        }

        gtk::Continue(true)
    }
}

fn render_async_record(async_: mi::AsyncRecord) -> String {
    let mut ret = String::new();
    ret.push_str(&format!("<b>{}</b> ", async_.class));
    let mut first = true;
    for (var, val) in async_.results {
        if !first {
            ret.push_str(", ");
        } else {
            first = false;
        }
        ret.push_str(&format!("{} = {}", var, render_value(&val)));
    }
    ret
}

fn render_value(val: &mi::Value) -> String {
    match val {
        mi::Value::Const(str) => escape_brackets(&str),
        mi::Value::Tuple(map) => {
            let mut ret = "{".to_string();
            let mut first = true;
            for (k, v) in map.iter() {
                if !first {
                    ret.push_str(", ");
                } else {
                    first = false;
                }
                ret.push_str(&format!("{} = {}", k, render_value(v)));
            }
            ret
        }
        mi::Value::ValueList(vals) => {
            let mut ret = "[".to_string();
            let mut first = true;
            for val in vals.iter() {
                if !first {
                    ret.push_str(", ");
                } else {
                    first = false;
                }
                ret.push_str(&render_value(val));
            }
            ret
        }
        mi::Value::ResultList(results) => {
            let mut ret = "[".to_string();
            let mut first = true;
            for (k, v) in results.iter() {
                if !first {
                    ret.push_str(", ");
                } else {
                    first = false;
                }
                ret.push_str(&format!("{} = {}", k, render_value(v)));
            }
            ret
        }
    }
}

/// Escape '<' and '>' characters in the string so that they don't look like pango tags when adding
/// to a text view. TODO: we should do proper HTML escaping
fn escape_brackets(s: &str) -> String {
    let mut ret = String::new();
    for c in s.chars() {
        if c == '<' {
            ret.push_str("&lt;");
        } else if c == '>' {
            ret.push_str("&gt;");
        } else {
            ret.push(c);
        }
    }
    ret
}
