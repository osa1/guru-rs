extern crate cairo;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;

mod app;
mod gdb;
mod mi;
mod parsers;
mod types;
mod widgets;

use gio::prelude::*;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let application =
        gtk::Application::new(None, Default::default()).expect("Initialization failed...");

    application.connect_startup(build_ui);
    application.connect_activate(|_| {});

    application.run(&[]);
}

fn build_ui(gtk_app: &gtk::Application) {
    let mut app = Rc::new(RefCell::new(app::App::new(gtk_app)));

    // Create gdb driver
    let (mut send, mut recv) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
    let gdb_driver = gdb::GDB::with_args(vec![], send);

    let main_context = glib::MainContext::default();
    {
        let app = app.clone();
        recv.attach(&main_context, move |msg| app.borrow_mut().mi_msg_recvd(msg));
    }
}
