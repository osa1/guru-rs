use gio::prelude::*;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::types::{Watchpoint, WatchpointType};
use crate::widgets::watchpoint_add::WatchpointAddW;

pub struct WatchpointsW {
    // scrolled -> box -> [tree view, button ("Add watchpoint")
    widget: gtk::ScrolledWindow,
    model: gtk::ListStore,
    view: gtk::TreeView,
    wp_enabled_renderer: gtk::CellRendererToggle,
    add_wp: WatchpointAddW,
}

/// Number of columns
const NUM_COLS: usize = 5;

/// Column indices for cell renderers
#[repr(i32)]
enum Cols {
    Enabled = 0,
    // Unique
    Number,
    // Expression, e.g. "((struct foo*)0x123)->bar"
    Expr,
    // Current value of the expression
    Value,
    // Number of hits so far
    Hits,
}

/// Column types for the list store
static COL_TYPES: [gtk::Type; NUM_COLS] = [
    gtk::Type::Bool,   // enabled
    gtk::Type::String, // number
    gtk::Type::String, // expr
    gtk::Type::String, // value
    gtk::Type::String, // hits
];

/// Column indices for when inserting rows into the list store
static COL_INDICES: [u32; NUM_COLS] = [0, 1, 2, 3, 4];

impl WatchpointsW {
    pub fn new() -> WatchpointsW {
        //
        // Create the store (model)
        //

        let model = gtk::ListStore::new(&COL_TYPES);

        //
        // Create the containers (scrolled, box)
        //

        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_.set_baseline_position(gtk::BaselinePosition::Top);
        scrolled.add(&box_);

        //
        // Create the "Add watchpoint" widget
        //

        let add_wp = WatchpointAddW::new();
        box_.pack_end(add_wp.get_widget(), false, false, 0);

        //
        // Create the view
        //

        let view = gtk::TreeView::new_with_model(&model);
        view.set_vexpand(false);
        view.set_hexpand(false);
        view.set_headers_visible(true);
        box_.pack_start(&view, true, true, 0);

        // Enabled column, render as a toggle
        let wp_enabled_renderer = gtk::CellRendererToggle::new();
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&wp_enabled_renderer, true);
        column.set_title("Enabled");
        column.add_attribute(&wp_enabled_renderer, "active", Cols::Enabled as i32);
        view.append_column(&column);

        // Helper for string columns
        let add_col = |title: &'static str, col_ty: Cols, editable: bool| {
            let renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            column.pack_start(&renderer, true);
            column.set_title(title);
            column.add_attribute(&renderer, "text", col_ty as i32);
            if editable {
                renderer.set_property_editable(true);
                renderer.connect_edited(|_w, _path, _str| { /* TODO */ });
            }
            // Finally add the column
            view.append_column(&column);
        };

        add_col("Number", Cols::Number, false);
        add_col("Expression", Cols::Expr, true);
        add_col("Value", Cols::Value, false);
        add_col("Hits", Cols::Hits, false);

        WatchpointsW {
            widget: scrolled,
            view,
            model,
            wp_enabled_renderer,
            add_wp,
        }
    }

    pub fn connect_watchpoint_added(&self, cb: Box<Fn(String, WatchpointType)>) {
        self.add_wp.connect_watchpoint_added(cb)
    }

    /// ONLY USE TO ADD THIS TO CONTAINERS!
    pub fn get_widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }

    pub fn add_watchpoint(&self, id: u32, expr: &str, type_: WatchpointType) {
        let values: [&dyn gtk::ToValue; NUM_COLS] = [
            &true.to_value(),
            &format!("#{}", id).to_value(),
            &expr.to_value(),
            &"".to_value(),
            &"0".to_value(),
        ];
        self.model.set(&self.model.append(), &COL_INDICES, &values);
    }
}
