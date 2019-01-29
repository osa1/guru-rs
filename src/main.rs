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
    let mut app = app::App::new(gtk_app);

    // Connect to gdb with no args in a few seconds, for testing
    gtk::timeout_add_seconds(3, move || {
        app.gdb_connect(vec![]);
        gtk::Continue(false)
    });
}
