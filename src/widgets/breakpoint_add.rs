//! A button for adding new breakpoints. When clicked it turns into two entries for location and
//! condition of the breakpoint. When submitted it turns back into a "add breakpoint" button.

use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

/// Type of a "breakpoint added" callback. Rc<RefCell<...>> becuase it's shared by entry "activate"
/// signal callbacks and the widget (to be able to set it after initializing all widgets).
type BreakpointAddCb = Rc<RefCell<Option<Box<Fn(String, String)>>>>;

pub struct BreakpointAddW {
    button: gtk::Button,
    cb: BreakpointAddCb,
}

impl BreakpointAddW {
    pub fn new() -> BreakpointAddW {
        //
        // Initialize the button
        //

        let button = gtk::Button::new_from_icon_name("gtk-add", gtk::IconSize::SmallToolbar);
        button.set_use_underline(true);
        button.set_label("New _breakpoint");
        button.set_halign(gtk::Align::Start);

        //
        // Initialize the entries
        //

        // grid -> [ [ location label, location entry],
        //           [ condition label, condition entry ] ]
        let grid = gtk::Grid::new();
        let location_label = gtk::Label::new("Location");
        let condition_label = gtk::Label::new("Condition");
        let location_entry = gtk::Entry::new();
        location_entry.set_hexpand(true);
        let condition_entry = gtk::Entry::new();
        condition_entry.set_hexpand(true);
        grid.attach(&location_label, 0, 0, 1, 1);
        grid.attach(&location_entry, 1, 0, 1, 1);
        grid.attach(&condition_label, 0, 1, 1, 1);
        grid.attach(&condition_entry, 1, 1, 1, 1);

        //
        // The callback cell
        //

        let cb: BreakpointAddCb = Rc::new(RefCell::new(None));

        //
        // Connect signals
        //

        // Button clicked -> remove the button and add the grid
        let grid_clone = grid.clone();
        let location_entry_clone = location_entry.clone();
        button.connect_clicked(move |w| {
            // Remove it from the parent
            let parent = w.get_parent().unwrap();
            let box_ = parent.downcast_ref::<gtk::Box>().unwrap();
            box_.remove(w);
            // Add the grid
            box_.pack_end(&grid_clone, false, false, 0);
            box_.show_all();
            // Move the focus to the location entry
            location_entry_clone.grab_focus();
        });

        // TODO: Remove duplication
        // Entry submitted -> call the callback, remove the grid, add button
        let condition_entry_clone = condition_entry.clone();
        let cb_clone = cb.clone();
        let grid_clone = grid.clone();
        let button_clone = button.clone();
        location_entry.connect_activate(move |w| {
            if let Some(location_str) = w.get_text() {
                if location_str.as_str().is_empty() {
                    return;
                }
                let location_str = location_str.as_str().to_string();
                let condition_str = condition_entry_clone
                    .get_text()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "".to_string());
                w.set_text("");
                condition_entry_clone.set_text("");
                if let Some(ref cb) = *cb_clone.borrow() {
                    cb(location_str, condition_str);
                }
                // Remove the grid from the parent
                let parent = w.get_ancestor(gtk::Box::static_type()).unwrap();
                let box_ = parent.downcast_ref::<gtk::Box>().unwrap();
                box_.remove(&grid_clone);
                // Add button
                box_.pack_end(&button_clone, false, false, 0);
                box_.show_all();
            }
        });

        // Same stuff for condition entry
        let location_entry_clone = location_entry.clone();
        let cb_clone = cb.clone();
        let grid_clone = grid.clone();
        let button_clone = button.clone();
        condition_entry.connect_activate(move |w| {
            if let Some(location_str) = location_entry_clone.get_text() {
                if location_str.as_str().is_empty() {
                    return;
                }
                let location_str = location_str.as_str().to_string();
                let condition_str = w
                    .get_text()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "".to_string());
                location_entry_clone.set_text("");
                w.set_text("");
                if let Some(ref cb) = *cb_clone.borrow() {
                    cb(location_str, condition_str);
                }
                // Remove the grid from the box
                let parent = w.get_ancestor(gtk::Box::static_type()).unwrap();
                let box_ = parent.downcast_ref::<gtk::Box>().unwrap();
                box_.remove(&grid_clone);
                // Add button
                box_.pack_end(&button_clone, false, false, 0);
                box_.show_all();
            }
        });

        BreakpointAddW { button, cb }
    }

    pub fn get_widget(&self) -> &gtk::Widget {
        // Return the button. When clicked it'll be removed and the grid will be added.
        self.button.upcast_ref()
    }

    /// Set "breakpoint added" callback. Arguments are: location, condition.
    pub fn connect_breakpoint_added(&self, cb: Box<Fn(String, String)>) {
        *self.cb.borrow_mut() = Some(cb);
    }
}
