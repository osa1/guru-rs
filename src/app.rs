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
    threads_w: RefCell<widgets::ThreadsW>,
    breakpoints_w: RefCell<widgets::BreakpointsW>,
    // watchpoints_w: widgets::WatchpointsW,
    expressions_w: RefCell<widgets::ExpressionsW>,
    gdb_w: RefCell<widgets::GdbW>,
    // GDB driver
    gdb: RefCell<Option<gdb::GDB>>,
    token: RefCell<u64>, // Maybe use an atomic type?
    callbacks: RefCell<HashMap<u64, Box<Fn(&AppInner, &App, mi::Result)>>>,
}

#[derive(Clone)]
pub struct App(Rc<AppInner>);

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

        let threads_w = widgets::ThreadsW::new();
        horiz1.pack2(threads_w.get_widget(), true, true);

        window.show_all();
        let app = App(Rc::new(AppInner {
            threads_w: RefCell::new(threads_w),
            breakpoints_w: RefCell::new(breakpoints_w),
            // watchpoints_w,
            expressions_w: RefCell::new(expressions_w),
            gdb_w: RefCell::new(gdb_w),
            gdb: RefCell::new(None),
            token: RefCell::new(0),
            callbacks: RefCell::new(HashMap::new()),
        }));

        //
        // Connect "breakpoint enabled" (the toggle buttons in breakpoint list)
        //

        {
            let app_clone = app.clone();
            app.0
                .breakpoints_w
                .borrow_mut()
                .connect_breakpoint_enabled(Box::new(move |bp_id, enable| {
                    app_clone.0.breakpoint_toggled(bp_id, enable);
                }));
        }

        //
        // Connect "breakpoint added" (the "add breakpoint" form in the breakpoint list)
        //

        {
            let app_clone = app.clone();
            app.0
                .breakpoints_w
                .borrow_mut()
                .connect_breakpoint_added(Box::new(move |location, condition| {
                    app_clone.0.breakpoint_added(location, condition);
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
                .gdb_w
                .borrow_mut()
                .connect_text_entered(move |msg| app_clone.send_mi_msg(msg));
        }

        //
        // Connect "add expression" (expressions widget)
        //

        {
            let app_clone = app.clone();
            app.0
                .expressions_w
                .borrow_mut()
                .connect_add_expr(Box::new(move |expr| app_clone.0.create_expr(expr)));
        }

        //
        // Connect "get children" (expressions widget)
        //

        {
            let app_clone = app.clone();
            app.0
                .expressions_w
                .borrow_mut()
                .connect_get_children(Box::new(move |name| {
                    app_clone.0.get_expr_children(name);
                }));
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
        *self.0.gdb.borrow_mut() = Some(gdb);
        self.0.gdb_w.borrow().enter_connected_state();
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
        self.0.gdb_w.borrow_mut().insert_line(&format!(
            "<span color=\"#6BDEB1\">[RESULT]</span> {}",
            render_result(&result)
        ));
        self.0.handle_result(self, result);
    }

    fn mi_oob_recvd(&self, oob: mi::OutOfBandResult) {
        match oob {
            mi::OutOfBandResult::ExecAsyncRecord(async_) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#505B70\">[EXEC]</span> {}",
                    render_async_record(&async_)
                ));
                self.0.handle_async_result(self, async_);
            }
            mi::OutOfBandResult::StatusAsyncRecord(async_) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#3FBCA6\">[STATUS]</span> {}",
                    render_async_record(&async_)
                ));
                self.0.handle_async_result(self, async_);
            }
            mi::OutOfBandResult::NotifyAsyncRecord(async_) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#CBCE79\">[NOTIFY]</span> {}",
                    render_async_record(&async_)
                ));
                self.0.handle_async_result(self, async_);
            }
            mi::OutOfBandResult::ConsoleStreamRecord(str) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#A1D490\">[CONSOLE]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
            mi::OutOfBandResult::TargetStreamRecord(str) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#90C3D4\">[TARGET]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
            mi::OutOfBandResult::LogStreamRecord(str) => {
                self.0.gdb_w.borrow().insert_line(&format!(
                    "<span color=\"#D4A190\">[LOG]</span> {}",
                    glib::markup_escape_text(&str)
                ));
            }
        }
    }

    pub fn send_mi_msg(&self, msg: String) {
        match *self.0.gdb.borrow_mut() {
            None => {
                // This should be a bug as the entry should be disabled when we're not connected
                println!("Can't send mi msg! GDB not available!");
            }
            Some(ref mut gdb) => {
                writeln!(gdb.stdin(), "{}", msg).unwrap();
                self.0.gdb_w.borrow().insert_line(&format!(">>> {}", msg));
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

macro_rules! some_ret {
    ( $x:expr, $ret:expr ) => {
        match $x {
            Some(ret) => ret,
            None => {
                return $ret;
            }
        }
    };
}

impl AppInner {
    fn get_token(&self) -> u64 {
        let mut token_ref = self.token.borrow_mut();
        let ret = *token_ref;
        *token_ref = ret + 1;
        ret
    }

    fn create_expr(&self, expr_str: String) {
        let token = self.get_token();
        let mut gdb_ref = self.gdb.borrow_mut();
        if let Some(ref mut gdb) = *gdb_ref {
            let stdin = gdb.stdin();
            writeln!(stdin, "{}-var-create - @ {}", token, expr_str).unwrap();
            drop(gdb_ref);
            self.callbacks.borrow_mut().insert(
                token,
                Box::new(move |app_inner, _app, result| {
                    if result.class != mi::ResultClass::Done {
                        println!("Error: {:?}", result);
                        return;
                    }
                    let expr = result.results;
                    match parsers::parse_var_create_result(expr) {
                        None => {
                            println!("Can't parse expression");
                        }
                        Some(expr) => {
                            app_inner.expressions_w.borrow_mut().add(
                                expr.name,
                                expr_str.to_owned(),
                                expr.value,
                                expr.type_,
                                expr.n_children != 0,
                            );
                        }
                    }
                }),
            );
        }
    }

    fn get_expr_children(&self, name: &str) {
        let token = self.get_token();
        let mut gdb_ref = self.gdb.borrow_mut();
        if let Some(ref mut gdb) = *gdb_ref {
            let stdin = gdb.stdin();
            writeln!(stdin, "{}-var-list-children --all-values {}", token, name).unwrap();
            drop(gdb_ref);

            self.callbacks.borrow_mut().insert(
                token,
                Box::new(move |app_inner, _app, mut result| {
                    if result.class != mi::ResultClass::Done {
                        println!("Error: {:?}", result);
                        return;
                    }
                    match parsers::parse_var_list_children_result(result.results) {
                        None => {
                            println!("Can't parse children list");
                            return;
                        }
                        Some(exprs) => {
                            for expr in exprs {
                                app_inner.expressions_w.borrow_mut().add(
                                    expr.name,
                                    expr.expr.unwrap(),
                                    expr.value,
                                    expr.type_,
                                    expr.n_children != 0,
                                )
                            }
                        }
                    }
                }),
            );
        }
    }

    fn breakpoint_toggled(&self, bp_id: u32, enable: bool) {
        // TODO: We should get token if gdb is available, but can't move this below as it borrowchk
        // still not smart enough.
        let token = self.get_token();
        let mut gdb_ref = self.gdb.borrow_mut();
        if let Some(ref mut gdb) = *gdb_ref {
            let stdin = gdb.stdin();
            if enable {
                writeln!(stdin, "{}-break-enable {}", token, bp_id).unwrap();
            } else {
                writeln!(stdin, "{}-break-disable {}", token, bp_id).unwrap();
            }
            drop(gdb_ref);
            self.callbacks.borrow_mut().insert(
                token,
                Box::new(move |app_inner, _app, _result| {
                    // TODO: Check if the result class is "Done"
                    app_inner
                        .breakpoints_w
                        .borrow_mut()
                        .toggle_breakpoint(bp_id, enable)
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

    fn breakpoint_added(&self, location: String, condition: String) {
        // TODO: Same as above, we need token only if gdb is available
        let token = self.get_token();
        let mut gdb_ref = self.gdb.borrow_mut();
        if let Some(ref mut gdb) = *gdb_ref {
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
            drop(gdb_ref);
            self.callbacks.borrow_mut().insert(
                token,
                Box::new(move |app_inner, _app, result| {
                    let results = result.results;
                    let bkpt = some!(parsers::parse_break_insert_result(results));
                    app_inner
                        .breakpoints_w
                        .borrow_mut()
                        .add_or_update_breakpoint(&bkpt);
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

    fn handle_result(&self, outer: &App, result: mi::Result) {
        if let Some(ref token) = result.token {
            let cb = self.callbacks.borrow_mut().remove(&token);
            match cb {
                None => {
                    println!("Can't find callback for result {}", token);
                }
                Some(cb) => {
                    cb(self, outer, result);
                }
            }
        }
    }

    // true -> execution stopped, false -> something else
    fn handle_async_result(&self, _outer: &App, mut async_: mi::AsyncRecord) {
        match async_.class.as_str() {
            "breakpoint-created" | "breakpoint-modified" => {
                let bkpt = some!(async_.results.remove("bkpt"));
                let bkpt = some!(bkpt.get_tuple());
                let bkpt = some!(parsers::parse_breakpoint(bkpt));
                self.breakpoints_w
                    .borrow_mut()
                    .add_or_update_breakpoint(&bkpt);
            }
            "stopped" => {
                // Execution stopped. Update threads.
                let token = self.get_token();
                let mut gdb_ref = self.gdb.borrow_mut();
                let mut gdb = gdb_ref.as_mut().unwrap();
                writeln!(gdb.stdin(), "{}-thread-info", token).unwrap();
                let token = self.get_token();
                self.threads_w.borrow_mut().clear();
                self.callbacks
                    .borrow_mut()
                    .insert(token, Box::new(thread_info_cb));
                // Update expressions
                writeln!(gdb.stdin(), "{}-var-update --all-values *", token).unwrap();
                drop(gdb_ref);
                self.callbacks
                    .borrow_mut()
                    .insert(token, Box::new(var_update_cb));
            }
            _ => {}
        }
    }
}

fn thread_info_cb(inner: &AppInner, _outer: &App, mut result: mi::Result) {
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
        let mut gdb_ref = inner.gdb.borrow_mut();
        let mut gdb = gdb_ref.as_mut().unwrap();
        writeln!(
            gdb.stdin(),
            "{}-stack-list-frames --thread {}",
            token,
            thread_id
        )
        .unwrap();
        inner.callbacks.borrow_mut().insert(
            token,
            Box::new(move |inner, outer, result| {
                thread_stack_cb(inner, outer, result, thread_id, &target_id)
            }),
        );
    }
}

fn var_update_cb(inner: &AppInner, _outer: &App, mut result: mi::Result) {
    println!("var_update_cb: {:?}", result);
    if result.class != mi::ResultClass::Done {
        return;
    }
    let changelist = result
        .results
        .remove("changelist")
        .unwrap()
        .get_value_list()
        .unwrap();

    let mut expressions_w = inner.expressions_w.borrow_mut();
    for change in changelist {
        let mut tuple = change.get_tuple().unwrap();
        let name = tuple.remove("name").unwrap().get_const().unwrap();
        let value = tuple.remove("value").unwrap().get_const().unwrap();
        println!("{} -> {}", name, value);
        expressions_w.update_value(name, value);
    }
}

fn thread_stack_cb(
    inner: &AppInner,
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
    inner
        .threads_w
        .borrow_mut()
        .add_thread(thread_id, target_id, &bt);
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
