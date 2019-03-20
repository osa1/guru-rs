//! A widget to show raw GDB output and to send input directly to it.

use gtk::prelude::*;

// TODOs:
//
// - The entry has more spacing around it than the text view. The entry looks better so we should
//   make the text view use same amount of spacing.
// - When the entry is selected and I press "up" it selects the text view, which is good. But if
//   I'm at the last line of the text view and press "down" it doesn't select the entry.
// - Adjust font size with ctrl+mouse scroll.

pub struct GdbW {
    // expander -> box -> [ scrolled -> text view, entry ]
    widget: gtk::Expander,
    text_view: gtk::TextView,
    entry: gtk::Entry,
}

// CSS for the entry
static ENTRY_STYLE: &'static str = "
    .monospace {
        font-family: monospace;
    }
    entry {
        border-top-style: dashed;
        border-right-style: none;
        border-bottom-style: none;
        border-left-style: none;
    }
";

impl GdbW {
    pub fn new() -> GdbW {
        let expander = gtk::Expander::new("GDB");
        expander.set_expanded(true);

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        expander.add(&box_);

        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let text_view = gtk::TextView::new();
        text_view.set_monospace(true);
        text_view.set_vexpand(true);
        text_view.set_editable(false);
        scrolled.add(&text_view);
        box_.pack_start(&scrolled, true, true, 0);

        // Add stuff for testing
        // let mut end_iter = text_view.get_buffer().unwrap().get_end_iter();
        // text_view.get_buffer().unwrap().insert(&mut end_iter, "testing");

        let entry = gtk::Entry::new();
        entry.set_vexpand(false);
        entry.set_placeholder_text("(enter gdb or gdb-mi commands here)");
        entry.set_sensitive(false);
        box_.pack_start(&entry, false, false, 0);

        //
        // Update entry style
        //

        let css_provider = gtk::CssProvider::new();
        css_provider
            .load_from_data(&ENTRY_STYLE.as_bytes())
            .unwrap();
        gtk::StyleContext::add_provider_for_screen(
            gdk::Screen::get_default().as_ref().unwrap(),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        entry.get_style_context().add_class("monospace");

        GdbW {
            widget: expander,
            text_view,
            entry,
        }
    }

    /// ONLY USE TO ADD THIS TO CONTAINERS!
    pub fn get_widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }

    pub fn insert_line(&self, str: &str) {
        // FIXME: Somehow this becomes false after first scrolling
        // let scroll_to_bottom = self.should_scroll_to_bottom();
        // println!("scroll_to_bottom: {}", scroll_to_bottom);

        let text_buffer = self.text_view.get_buffer().unwrap();

        // Insert the text
        let mut end_iter = text_buffer.get_end_iter();
        text_buffer.insert_markup(&mut end_iter, str.trim());
        let mut end_iter = text_buffer.get_end_iter();
        text_buffer.insert(&mut end_iter, "\n");

        self.scroll_to_bottom();
        // if scroll_to_bottom {
        //     self.scroll_to_bottom();
        //     assert!(self.should_scroll_to_bottom());
        // }
    }

    pub fn connect_text_entered<F: Fn(String) + 'static>(&self, f: F) {
        self.entry.connect_activate(move |entry| {
            if let Some(text) = entry.get_text() {
                let text = text.as_str().to_string();
                entry.set_text("");
                f(text);
            }
        });
    }

    pub fn enter_connected_state(&self) {
        self.entry.set_sensitive(true);
    }

    fn should_scroll_to_bottom(&self) -> bool {
        // If the user adjusted the scroll bar then we don't scroll to the end. Otherwise we do.
        // Code taken from https://mail.gnome.org/archives/gtk-list/2011-August/msg00034.html and I
        // don't understand how it works.
        let adjustment = self.text_view.get_vadjustment().unwrap();
        let value: f64 = adjustment.get_value();
        let upper: f64 = adjustment.get_upper();
        let page_size: f64 = adjustment.get_page_size();
        let rhs = upper - page_size - 1e-12;
        let ret = value >= rhs;
        println!(
            "value: {}, upper: {}, page_size: {} (upper - page_size - 1e-12): {}, ret {}",
            value, upper, page_size, rhs, ret
        );
        ret
    }

    fn scroll_to_bottom(&self) {
        let text_buffer = self.text_view.get_buffer().unwrap();
        let last_line_iter = text_buffer.get_iter_at_line(text_buffer.get_line_count());
        let last_line_mark = text_buffer
            .create_mark("end", &last_line_iter, false /* left gravity */)
            .unwrap();
        self.text_view.scroll_mark_onscreen(&last_line_mark);
    }
}
