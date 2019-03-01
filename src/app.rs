use crate::gdb;
use crate::mi;
use crate::parsers;
use crate::types::WatchpointType;
use crate::widgets;

use gtk::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

struct AppInner {
    // Widgets
    threads_w: widgets::ThreadsW,
    breakpoints_w: widgets::BreakpointsW,
    // watchpoints_w: widgets::WatchpointsW,
    expressions_w: widgets::ExpressionsW,
    gdb_w: widgets::GdbW,
    // GDB driver
    gdb: Option<gdb::GDB>,
    token: u64,
    callbacks: HashMap<u64, Box<Fn(&mut AppInner, &App, mi::Result)>>,
}

#[derive(Clone)]
pub struct App(Rc<RefCell<AppInner>>);

impl App {
    pub fn new(gtk_app: &gtk::Application) -> App {
        let window = gtk::ApplicationWindow::new(gtk_app);
        window.set_default_size(1200, 1050);
        window.set_title("guru");

        // Horizontal: | Vertical: -

        // Current layout:
        // horiz(1) ->
        //   [ vert(1) -> [ vert(2) -> [ currently_empty, gdb logs ],
        //                  flow box -> [ breakpoints, /* watchpoints */ expressions ] ]
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

        let flow_box = gtk::FlowBox::new();
        flow_box.set_homogeneous(false);
        flow_box.set_vexpand(true);
        flow_box.set_hexpand(true);
        flow_box.set_row_spacing(0);
        flow_box.set_column_spacing(0);
        flow_box.set_max_children_per_line(2);
        vert1.pack2(&flow_box, true, true);

        let breakpoints_w = widgets::BreakpointsW::new();
        flow_box.insert(breakpoints_w.get_widget(), 0);

        let mut expressions_w = widgets::ExpressionsW::new();
        flow_box.insert(expressions_w.get_widget(), 1);
        // let watchpoints_w = widgets::WatchpointsW::new();
        // flow_box.insert(watchpoints_w.get_widget(), 1);

        // Add some data for testing
        expressions_w.add(
            "var1".to_string(),
            "MainCapability".to_string(),
            None,
            "Capability".to_string(),
            true,
        );
        expressions_w.add(
            "var2".to_string(),
            "n_capabilities".to_string(),
            Some("0".to_string()),
            "unsigned int".to_string(),
            false,
        );
        expressions_w.add(
            "var1.f".to_string(),
            "f".to_string(),
            None,
            "StgFunTable".to_string(),
            true,
        );
        expressions_w.add(
            "var1.r".to_string(),
            "r".to_string(),
            None,
            "StgRegTable".to_string(),
            true,
        );
        expressions_w.add(
            "var1.r.x".to_string(),
            "x".to_string(),
            None,
            "int".to_string(),
            true,
        );
        // expressions_w.add(
        //     "var1.r.y".to_string(),
        //     "y".to_string(),
        //     None,
        //     "int".to_string(),
        //     true,
        // );

        let threads_w = widgets::ThreadsW::new();
        horiz1.pack2(threads_w.get_widget(), true, true);

        window.show_all();
        let app = App(Rc::new(RefCell::new(AppInner {
            threads_w,
            breakpoints_w,
            // watchpoints_w,
            expressions_w,
            gdb_w,
            gdb: None,
            token: 0,
            callbacks: HashMap::new(),
        })));

        //
        // Connect "breakpoint enabled" (the toggle buttons in breakpoint list)
        //

        {
            let app_clone = app.clone();
            app.0
                .borrow_mut()
                .breakpoints_w
                .connect_breakpoint_enabled(Box::new(move |bp_id, enable| {
                    app_clone.0.borrow_mut().breakpoint_toggled(bp_id, enable);
                }));
        }

        //
        // Connect "breakpoint added" (the "add breakpoint" form in the breakpoint list)
        //

        {
            let app_clone = app.clone();
            app.0
                .borrow_mut()
                .breakpoints_w
                .connect_breakpoint_added(Box::new(move |location, condition| {
                    app_clone
                        .0
                        .borrow_mut()
                        .breakpoint_added(location, condition);
                }));
        }

        //
        // Connect "watchpoint enabled" (the toggle buttons in watchpoint list)
        //

        /*
        {
            let app_clone = app.clone();
            app.0
                .borrow_mut()
                .watchpoints_w
                .connect_watchpoint_enabled(Box::new(move |wp_id, enable| {
                    app_clone.0.borrow_mut().watchpoint_toggled(wp_id, enable);
                }));
        }
        */

        //
        // Connect "watchpoint added" (the "watchpoint breakpoint" form in the watchpoint list)
        //

        /*
        {
            let app_clone = app.clone();
            app.0
                .borrow_mut()
                .watchpoints_w
                .connect_watchpoint_added(Box::new(move |expr, type_| {
                    app_clone.0.borrow_mut().watchpoint_added(expr, type_);
                }));
        }
        */

        //
        // Connect gdb raw input entry
        //

        {
            let app_clone = app.clone();
            app.0
                .borrow_mut()
                .gdb_w
                .connect_text_entered(move |msg| app_clone.send_mi_msg(msg));
        }

        app
    }

    pub fn gdb_connect(&self, args: &[String]) {
        let (send, recv) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
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
        for msg in mi_msgs {
            match msg {
                mi::ResultOrOOB::Result(result) => self.mi_result_recvd(result),
                mi::ResultOrOOB::OOB(oob) => self.mi_oob_recvd(oob),
            }
        }
        gtk::Continue(true)
    }

    fn mi_result_recvd(&self, result: mi::Result) {
        let mut inner = self.0.borrow_mut();
        inner.gdb_w.insert_line(&format!(
            "<span color=\"#6BDEB1\">[RESULT]</span> {}",
            render_result(&result)
        ));
        inner.handle_result(self, result);
    }

    fn mi_oob_recvd(&self, oob: mi::OutOfBandResult) {
        let mut inner = self.0.borrow_mut();
        match oob {
            mi::OutOfBandResult::ExecAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#505B70\">[EXEC]</span> {}",
                    render_async_record(&async_)
                ));
                inner.handle_async_result(self, async_);
            }
            mi::OutOfBandResult::StatusAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#3FBCA6\">[STATUS]</span> {}",
                    render_async_record(&async_)
                ));
                inner.handle_async_result(self, async_);
            }
            mi::OutOfBandResult::NotifyAsyncRecord(async_) => {
                inner.gdb_w.insert_line(&format!(
                    "<span color=\"#CBCE79\">[NOTIFY]</span> {}",
                    render_async_record(&async_)
                ));
                inner.handle_async_result(self, async_);
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

    pub fn send_mi_msg(&self, msg: String) {
        let mut inner = self.0.borrow_mut();
        match inner.gdb.as_mut() {
            None => {
                // This should be a bug as the entry should be disabled when we're not connected
                println!("Can't send mi msg! GDB not available!");
            }
            Some(gdb) => {
                writeln!(gdb.stdin(), "{}", msg).unwrap();
                inner.gdb_w.insert_line(&format!(">>> {}", msg));
                // let _ = gdb.stdin().flush();
            }
        }
    }
}

// TODO find a better name
macro_rules! some {
    ( $x:expr ) => {
        match $x {
            Some(ret) => ret,
            None => {
                return;
            }
        }
    };
}

impl AppInner {
    fn get_token(&mut self) -> u64 {
        let ret = self.token;
        self.token += 1;
        ret
    }

    fn breakpoint_toggled(&mut self, bp_id: u32, enable: bool) {
        // TODO: We should get token if gdb is available, but can't move this below as it borrowchk
        // still not smart enough.
        let token = self.get_token();
        if let Some(ref mut gdb) = self.gdb {
            let stdin = gdb.stdin();
            if enable {
                writeln!(stdin, "{}-break-enable {}", token, bp_id).unwrap();
            } else {
                writeln!(stdin, "{}-break-disable {}", token, bp_id).unwrap();
            }
            self.callbacks.insert(
                token,
                Box::new(move |app_inner, _app, _result| {
                    // TODO: Check if the result class is "Done"
                    app_inner.breakpoints_w.toggle_breakpoint(bp_id, enable)
                }),
            );
        }
    }

    /*
    fn watchpoint_toggled(&mut self, bp_id: u32, enable: bool) {
        // TODO: We should get token if gdb is available, but can't move this below as it borrowchk
        // still not smart enough.
        let token = self.get_token();
        if let Some(ref mut gdb) = self.gdb {
            let stdin = gdb.stdin();
            if enable {
                writeln!(stdin, "{}-break-enable {}", token, bp_id).unwrap();
            } else {
                writeln!(stdin, "{}-break-disable {}", token, bp_id).unwrap();
            }
            self.callbacks.insert(
                token,
                Box::new(move |app_inner, _app, _result| {
                    // TODO: Check if the result class is "Done"
                    app_inner.watchpoints_w.toggle_watchpoint(bp_id, enable)
                }),
            );
        }
    }
    */

    fn breakpoint_added(&mut self, location: String, condition: String) {
        // TODO: Same as above, we need token only if gdb is available
        let token = self.get_token();
        if let Some(ref mut gdb) = self.gdb {
            let stdin = gdb.stdin();
            if condition.is_empty() {
                writeln!(stdin, "{}-break-insert {}", token, location).unwrap();
            } else {
                writeln!(
                    stdin,
                    "{}-break-insert -c \"{}\" {}",
                    token, condition, location
                )
                .unwrap();
            }
            self.callbacks.insert(
                token,
                Box::new(move |app_inner, _app, result| {
                    let mut results = result.results;
                    // TODO handle errors
                    let bkpt = some!(results.remove("bkpt"));
                    let bkpt = some!(bkpt.get_tuple());
                    let bkpt = some!(parsers::parse_breakpoint(bkpt));
                    app_inner.breakpoints_w.add_or_update_breakpoint(&bkpt);
                }),
            );
        }
    }

    /*
    fn watchpoint_added(&mut self, expr: String, type_: WatchpointType) {
        // TODO: Same as above
        let token = self.get_token();
        if let Some(ref mut gdb) = self.gdb {
            let mode = match type_ {
                WatchpointType::ReadWrite => "-a",
                WatchpointType::Read => "-r",
                WatchpointType::Write => "",
            };
            writeln!(gdb.stdin(), "{}-break-watch {} \"{}\"", token, mode, expr).unwrap();
            self.callbacks.insert(
                token,
                Box::new(move |app_inner, _app, result| {
                    if result.class == mi::ResultClass::Done {
                        // This message doesn't have enough information so we move the info from
                        // the args
                        let results = result.results;
                        for (_k, v) in results.into_iter() {
                            if let Some(mut tuple) = v.get_tuple() {
                                if let Some(id) = tuple.remove("number") {
                                    let id = some!(some!(id.get_const()).parse::<u32>().ok());
                                    app_inner.watchpoints_w.add_watchpoint(
                                        id,
                                        expr.as_str(),
                                        type_,
                                    );
                                    return;
                                }
                            }
                        }
                    }
                }),
            );
        }
    }
    */

    fn handle_result(&mut self, outer: &App, result: mi::Result) {
        if let Some(ref token) = result.token {
            match self.callbacks.remove(&token) {
                None => {
                    println!("Can't find callback for result {}", token);
                }
                Some(cb) => {
                    cb(self, outer, result);
                }
            }
        }
    }

    fn handle_async_result(&mut self, _outer: &App, mut async_: mi::AsyncRecord) {
        match async_.class.as_str() {
            "breakpoint-created" | "breakpoint-modified" => {
                let bkpt = some!(async_.results.remove("bkpt"));
                let bkpt = some!(bkpt.get_tuple());
                let bkpt = some!(parsers::parse_breakpoint(bkpt));
                self.breakpoints_w.add_or_update_breakpoint(&bkpt);
            }
            "stopped" => {
                // Execution stopped. Update threads.
                let token = self.get_token();
                let gdb = some!(self.gdb.as_mut());
                writeln!(gdb.stdin(), "{}-thread-info", token).unwrap();
                self.threads_w.clear();
                self.callbacks.insert(token, Box::new(thread_info_cb));
            }
            _ => {}
        }
    }
}

fn thread_info_cb(inner: &mut AppInner, _outer: &App, mut result: mi::Result) {
    // [RESULT] Done: current-thread-id = 1, threads = [{core = 4, frame = {level = 0, file = ../sysdeps/unix/sysv/linux/write.c, fullname = /build/glibc-OTsEL5/glibc-2.27/nptl/../sysdeps/unix/sysv/linux/write.c, func = __libc_write, addr = 0x00007ffff591e2b7, args = [{value = 11, name = fd}, {value = 0x555555d44860, name = buf}, {value = 4, name = nbytes}], line = 27}, state = stopped, target-id = Thread 0x7ffff7fbdb80 (LWP 19785), id = 1, name = guru}, {id = 2, target-id = Thread 0x7fffed538700 (LWP 19789), frame = {fullname = /build/glibc-OTsEL5/glibc-2.27/io/../sysdeps/unix/sysv/linux/poll.c, addr = 0x00007ffff5418bf9, func = __GI___poll, file = ../sysdeps/unix/sysv/linux/poll.c, args = [{value = 0x55555592e740, name = fds}, {value = 1, name = nfds}, {name = timeout, value = -1}], line = 29, level = 0}, state = stopped, core = 4, name = gmain}, {name = gdbus, state = stopped, target-id = Thread 0x7fffecd37700 (LWP 19790), id = 3, frame = {level = 0, func = __GI___poll, line = 29, args = [{value = 0x555555942bf0, name = fds}, {value = 2, name = nfds}, {value = -1, name = timeout}], addr = 0x00007ffff5418bf9, file = ../sysdeps/unix/sysv/linux/poll.c, fullname = /build/glibc-OTsEL5/glibc-2.27/io/../sysdeps/unix/sysv/linux/poll.c}, core = 1}, {target-id = Thread 0x7fffe778e700 (LWP 19792), core = 7, id = 5, name = pool, frame = {args = [], func = syscall, level = 0, file = ../sysdeps/unix/sysv/linux/x86_64/syscall.S, fullname = /build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/syscall.S, addr = 0x00007ffff541f839, line = 38}, state = stopped}]
    if result.class != mi::ResultClass::Done {
        return;
    }
    let threads = result
        .results
        .remove("threads")
        .unwrap()
        .get_value_list()
        .unwrap();
    for thread in threads {
        let mut thread = thread.get_tuple().unwrap();
        let thread_id =
            str::parse::<i32>(thread.remove("id").unwrap().get_const_ref().unwrap()).unwrap();
        let target_id = thread.remove("target-id").unwrap().get_const().unwrap();
        let token = inner.get_token();
        let gdb = inner.gdb.as_mut().unwrap();
        writeln!(
            gdb.stdin(),
            "{}-stack-list-frames --thread {}",
            token,
            thread_id
        )
        .unwrap();
        inner.callbacks.insert(
            token,
            Box::new(move |inner, outer, result| {
                thread_stack_cb(inner, outer, result, thread_id, &target_id)
            }),
        );
    }
}

fn thread_stack_cb(
    inner: &mut AppInner,
    _outer: &App,
    mut result: mi::Result,
    thread_id: i32,
    target_id: &str,
) {
    // [RESULT] Done: stack = [frame = {file = ../sysdeps/unix/sysv/linux/x86_64/syscall.S, func = syscall, level = 0, line = 38, addr = 0x00007ffff541f839, fullname = /build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/syscall.S}, frame = {from = /usr/lib/x86_64-linux-gnu/libglib-2.0.so.0, level = 1, addr = 0x00007ffff5fca29a, func = g_cond_wait_until}, frame = {from = /usr/lib/x86_64-linux-gnu/libglib-2.0.so.0, addr = 0x00007ffff5f574f1, level = 2, func = ??}, frame = {func = g_async_queue_timeout_pop, from = /usr/lib/x86_64-linux-gnu/libglib-2.0.so.0, level = 3, addr = 0x00007ffff5f57aac}, frame = {func = ??, from = /usr/lib/x86_64-linux-gnu/libglib-2.0.so.0, level = 4, addr = 0x00007ffff5facbae}, frame = {from = /usr/lib/x86_64-linux-gnu/libglib-2.0.so.0, func = ??, addr = 0x00007ffff5fac105, level = 5}, frame = {fullname = /build/glibc-OTsEL5/glibc-2.27/nptl/pthread_create.c, level = 6, line = 463, func = start_thread, file = pthread_create.c, addr = 0x00007ffff59146db}, frame = {file = ../sysdeps/unix/sysv/linux/x86_64/clone.S, func = clone, addr = 0x00007ffff542588f, line = 95, fullname = /build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/clone.S, level = 7}
    let bt = result
        .results
        .remove("stack")
        .unwrap()
        .get_result_list()
        .unwrap();
    let bt = parsers::parse_backtrace(bt).unwrap();
    inner.threads_w.add_thread(thread_id, target_id, &bt);
    // TODO: Doing this on every update is not a good idea!
    // inner.threads_w.reset_cols();
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
        mi::ResultClass::Done => "Done",
        mi::ResultClass::Running => "Running",
        mi::ResultClass::Connected => "Connected",
        mi::ResultClass::Error => "Error",
        mi::ResultClass::Exit => "Exit",
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
