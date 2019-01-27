extern crate cairo;
extern crate gdk;
extern crate gio;
extern crate gtk;

mod mi;
mod types;
mod widgets;

use gtk::prelude::*;
use gio::prelude::*;

fn main() {
    let application =
        gtk::Application::new(None, Default::default())
            .expect("Initialization failed...");

    application.connect_startup(build_ui);
    application.connect_activate(|_| {});

    application.run(&::std::env::args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("guru");


    window.show_all();
}
