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

    // Currently all args are considered gdb args and passed to gdb as --args, e.g.
    // $ gdb --args <program args>
    let gdb_args = std::env::args()
        .into_iter()
        .skip(1)
        .collect::<Vec<String>>();
    println!("args: {:?}", gdb_args);

    // Connect to gdb with no args, for testing
    app.gdb_connect(&gdb_args);
}
