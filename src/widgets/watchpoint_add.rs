//! Similar to `BreakPointAddW`

use gio::prelude::*;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::types::WatchpointType;

/// Type of a "watchpoint added" callback. Rc<RefCell<...>> becuase it's shared by entry "activate"
/// signal callbacks and the widget (to be able to set it after initializing all widgets).
/// Arguments are: expression to watch, watchpoint type
type WatchpointAddCb = Rc<RefCell<Option<Box<Fn(String, WatchpointType)>>>>;

pub struct WatchpointAddW {
    button: gtk::Button,
    // grid -> [ [ expression label, expression entry],
    //           [ type label, combo box ] ]
    grid: gtk::Grid,
    cb: WatchpointAddCb,
}

impl WatchpointAddW {
    pub fn new() -> WatchpointAddW {
        //
        // Initialize the button
        //

        let button = gtk::Button::new_from_icon_name("gtk-add", gtk::IconSize::SmallToolbar);
        button.set_use_underline(true);
        button.set_label("New _watchpoint");
        button.set_halign(gtk::Align::Start);

        //
        // Initialize the entries
        //

        let grid = gtk::Grid::new();
        let expression_label = gtk::Label::new("Expression");
        let expression_entry = gtk::Entry::new();
        expression_entry.set_hexpand(true);
        let type_label = gtk::Label::new("Type");
        let type_combo = gtk::ComboBoxText::new();
        type_combo.append_text("Read/Write");
        type_combo.append_text("Read");
        type_combo.append_text("Write");
        type_combo.set_active(0);

        grid.attach(&expression_label, 0, 0, 1, 1);
        grid.attach(&expression_entry, 1, 0, 1, 1);
        grid.attach(&type_label, 0, 1, 1, 1);
        grid.attach(&type_combo, 1, 1, 1, 1);

        //
        // The callback cell
        //

        let cb: WatchpointAddCb = Rc::new(RefCell::new(None));

        //
        // Connect signals
        //

        // Button clicked -> remove the button and add the grid
        let grid_clone = grid.clone();
        let expression_entry_clone = expression_entry.clone();
        button.connect_clicked(move |w| {
            // Remove it from the parent
            let parent = w.get_parent().unwrap();
            let box_ = parent.downcast_ref::<gtk::Box>().unwrap();
            box_.remove(w);
            // Add the grid
            box_.pack_end(&grid_clone, false, false, 0);
            box_.show_all();
            // Move the focus to the location entry
            expression_entry_clone.grab_focus();
        });

        // Entry submitted -> call the callback, remove the grid, add button
        let cb_clone = cb.clone();
        let grid_clone = grid.clone();
        let type_combo_clone = type_combo.clone();
        let button_clone = button.clone();
        expression_entry.connect_activate(move |w| {
            if let Some(expression_str) = w.get_text() {
                if expression_str.as_str().is_empty() {
                    return;
                }
                let expression_str = expression_str.as_str().to_string();
                let wp_type = match type_combo_clone.get_active() {
                    Some(0) => WatchpointType::ReadWrite,
                    Some(1) => WatchpointType::Read,
                    Some(2) => WatchpointType::Write,
                    other => {
                        panic!("Unexpected active in watchpoint type combo: {:?}", other);
                    }
                };
                if let Some(ref cb) = *cb_clone.borrow() {
                    cb(expression_str, wp_type)
                }
                // Remove the grid from the box
                let parent = w.get_ancestor(gtk::Box::static_type()).unwrap();
                let box_ = parent.downcast_ref::<gtk::Box>().unwrap();
                box_.remove(&grid_clone);
                // Add button
                box_.pack_end(&button_clone, false, false, 0);
                box_.show_all();
            };
        });

        WatchpointAddW { button, grid, cb }
    }

    pub fn get_widget(&self) -> &gtk::Widget {
        // Return the button. When clicked it'll be removed and the grid will be added.
        self.button.upcast_ref()
    }

    pub fn connect_watchpoint_added(&self, cb: Box<Fn(String, WatchpointType)>) {
        *self.cb.borrow_mut() = Some(cb);
    }
}
