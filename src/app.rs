use crate::gdb;
use crate::mi;
use crate::parsers;
use crate::widgets;

use gio::prelude::*;
use gtk::prelude::*;

use std::cell::{RefCell, RefMut};
use std::io::Write;
use std::rc::Rc;

struct AppInner {
    // Widgets
    threads_w: widgets::ThreadsW,
    breakpoints_w: widgets::BreakpointsW,
    gdb_w: widgets::GdbW,
    // GDB driver
    gdb: Option<gdb::GDB>,
}

#[derive(Clone)]
pub struct App(Rc<RefCell<AppInner>>);

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

        let app = App(Rc::new(RefCell::new(AppInner {
            threads_w,
            breakpoints_w,
            gdb_w,
            gdb: None,
        })));

        {
            let app1 = app.clone();
            app.0
                .borrow_mut()
                .gdb_w
                .connect_text_entered(move |msg| app1.send_mi_msg(msg));
        }

        app
    }

    pub fn gdb_connect(&self, args: &[String]) {
        let (mut send, mut recv) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
        let gdb = gdb::GDB::with_args(args, send); // TODO errors
        let main_context = glib::MainContext::default();
        {
            let app = self.clone();
            recv.attach(&main_context, move |msg| app.mi_msg_recvd(msg));
        }
        // TODO error checking
        self.0.borrow_mut().gdb = Some(gdb);
        self.0.borrow().gdb_w.enter_connected_state();
    }

    pub fn mi_msg_recvd(&self, mi_msgs: mi::Output) -> gtk::Continue {
        let inner = self.0.borrow();
        for msg in mi_msgs {
            match msg {
                mi::ResultOrOOB::Result(result) => self.mi_result_recvd(result),
                mi::ResultOrOOB::OOB(oob) => self.mi_oob_recvd(oob),
            }
        }
        gtk::Continue(true)
    }

    fn mi_result_recvd(&self, result: mi::Result) {
        let inner = self.0.borrow();
        inner.gdb_w.insert_line(&format!(
            "<span color=\"#6BDEB1\">[RESULT]</span> {}",
            render_result(&result)
        ));
    }

    fn mi_oob_recvd(&self, oob: mi::OutOfBandResult) {
        let inner = self.0.borrow();
        match oob {
            mi::OutOfBandResult::ExecAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#505B70\">[EXEC]</span> {}",
                    render_async_record(&async_)
                ));
                self.handle_async_result(async_);
            }
            mi::OutOfBandResult::StatusAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#3FBCA6\">[STATUS]</span> {}",
                    render_async_record(&async_)
                ));
                self.handle_async_result(async_);
            }
            mi::OutOfBandResult::NotifyAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#CBCE79\">[NOTIFY]</span> {}",
                    render_async_record(&async_)
                ));
                self.handle_async_result(async_);
            }
            mi::OutOfBandResult::ConsoleStreamRecord(str) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#A1D490\">[CONSOLE]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
            mi::OutOfBandResult::TargetStreamRecord(str) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#90C3D4\">[TARGET]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
            mi::OutOfBandResult::LogStreamRecord(str) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#D4A190\">[LOG]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
        }
    }

    fn handle_async_result(&self, mut async_: mi::AsyncRecord) {
        // TODO find a better name
        macro_rules! some {
            ( $x:expr ) => {
                if let Some(ret) = $x {
                    ret
                } else {
                    return;
                }
            }
        };

        match async_.class.as_str() {
            "breakpoint-created" => {
                let bkpt = some!(async_.results.remove("bkpt"));
                let bkpt = some!(bkpt.get_tuple());
                let bkpt = some!(parsers::parse_breakpoint(bkpt));
                let inner = self.0.borrow();
                inner.breakpoints_w.add_breakpoint(&bkpt);
            }
            _ => {

            }
        }
    }

    pub fn send_mi_msg(&self, msg: String) {
        let mut inner = self.0.borrow_mut();
        match inner.gdb.as_mut() {
            None => {
                // This should be a bug as the entry should be disabled when we're not connected
                println!("Can't send mi msg! GDB not available!");
            }
            Some(mut gdb) => {
                writeln!(gdb.stdin(), "{}", msg).unwrap();
                inner.gdb_w.insert_line(&format!(">>> {}", msg));
                // let _ = gdb.stdin().flush();
            }
        }
    }
}

fn render_async_record(async_: &mi::AsyncRecord) -> String {
    let mut ret = String::new();
    ret.push_str(&format!("<b>{}</b> ", async_.class));
    let mut first = true;
    for (var, val) in async_.results.iter() {
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
        mi::Value::Const(str) => glib::markup_escape_text(&str).as_str().to_owned(),
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
            ret.push('}');
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
            ret.push(']');
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

fn render_result(result: &mi::Result) -> String {
    let mut ret = String::new();
    ret.push_str(match &result.class {
        Done => "Done",
        Running => "Running",
        Connected => "Connected",
        Error => "Error",
        Exit => "Exit",
    });
    if result.results.is_empty() {
        return ret;
    }
    ret.push_str(": ");
    let mut first = true;
    for (var, val) in result.results.iter() {
        if !first {
            ret.push_str(", ");
        }
        first = false;
        ret.push_str(&format!("{} = {}", var, render_value(val)));
    }
    ret
}
