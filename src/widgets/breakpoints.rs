//! A `TreeView` for rendering breakpoints.

use gio::prelude::*;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::types::{Breakpoint, BreakpointDisposition, BreakpointType};
use crate::widgets::breakpoint_add::BreakpointAddW;

pub struct BreakpointsW {
    // scrolled -> box -> [tree view, button ("Add breakpoints")]
    widget: gtk::ScrolledWindow,
    model: gtk::ListStore,
    view: gtk::TreeView,
    bp_enabled_renderer: gtk::CellRendererToggle,
    // "Add breakpoint" widget
    add_bp: BreakpointAddW,
}

// TODO: How to best show disposition?

/// Number of columns
const NUM_COLS: usize = 7;

/// Column indices for cell renderers
#[repr(i32)]
enum Cols {
    Enabled = 0,
    // Unique
    Number,
    // Usually just a function name
    Location,
    // E.g. foo.c:123
    File,
    // Memory location ($rip) of the breakpoint
    Address,
    // Condition
    Cond,
    // Number of hits so far
    Hits,
}

/// Column types for the list store
static COL_TYPES: [gtk::Type; NUM_COLS] = [
    gtk::Type::Bool,   // enabled
    gtk::Type::String, // number
    gtk::Type::String, // location
    gtk::Type::String, // file
    gtk::Type::String, // address
    gtk::Type::String, // condition
    gtk::Type::String, // hits
];

/// Column indices for when inserting rows into the list store
static COL_INDICES: [u32; NUM_COLS] = [0, 1, 2, 3, 4, 5, 6];

impl BreakpointsW {
    pub fn new() -> BreakpointsW {
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
        // Create the "Add breakpoint" widget
        //

        let add_bp = BreakpointAddW::new();
        box_.pack_end(add_bp.get_widget(), false, false, 0);

        //
        // Create the view
        //

        let view = gtk::TreeView::new_with_model(&model);
        view.set_vexpand(false);
        view.set_hexpand(false);
        view.set_headers_visible(true);
        box_.pack_start(&view, true, true, 0);

        // Enabled column, render as a toggle
        let bp_enabled_renderer = gtk::CellRendererToggle::new();
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&bp_enabled_renderer, true);
        column.set_title("Enabled");
        column.add_attribute(&bp_enabled_renderer, "active", Cols::Enabled as i32);
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
        add_col("Location", Cols::Location, true);
        add_col("File", Cols::File, true);
        add_col("Address", Cols::Address, false);
        add_col("Condition", Cols::Cond, true);
        add_col("Hits", Cols::Hits, false);

        BreakpointsW {
            widget: scrolled,
            view,
            model,
            bp_enabled_renderer,
            add_bp,
        }
    }

    pub fn connect_breakpoint_enabled(
        &self,
        cb: Box<Fn(u32, bool /* true => enable, false => disable */)>,
    ) {
        let model = self.model.clone(); // TODO: I hope this is just a refcount bump?
        self.bp_enabled_renderer.connect_toggled(move |_w, path| {
            let iter = model.get_iter(&path).unwrap();
            let old_enabled = model
                .get_value(&iter, Cols::Enabled as i32)
                .get::<bool>()
                .unwrap();
            let bp_id = model
                .get_value(&iter, Cols::Number as i32)
                .get::<String>()
                .unwrap()
                .parse::<u32>()
                .unwrap();
            cb(bp_id, !old_enabled);
        });
    }

    /// Set "breakpoint added" callback. Arguments are: location, condition.
    pub fn connect_breakpoint_added(&self, cb: Box<Fn(String, String)>) {
        self.add_bp.connect_breakpoint_added(cb);
    }

    pub fn toggle_breakpoint(&self, bp_id: u32, enable: bool) {
        // find the row for the row with given breakpoint id
        if let Some(iter) = self.model.get_iter_first() {
            loop {
                let bp_id_ = self
                    .model
                    .get_value(&iter, Cols::Number as i32)
                    .get::<String>()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                if bp_id_ == bp_id {
                    self.model
                        .set_value(&iter, Cols::Enabled as u32, &enable.to_value());
                    return;
                }
                if !self.model.iter_next(&iter) {
                    break;
                }
            }
        }
    }

    /// ONLY USE TO ADD THIS TO CONTAINERS!
    pub fn get_widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }

    /// Update the breakpoint if it exists, otherwise add a new one.
    pub fn add_or_update_breakpoint(&self, bp: &Breakpoint) {
        println!("add_or_update_breakpoint({:?})", bp);
        if let Some(iter) = self.model.get_iter_first() {
            loop {
                let bp_id = self
                    .model
                    .get_value(&iter, Cols::Number as i32)
                    .get::<String>()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                if bp_id == bp.number {
                    self.model.set(
                        &iter,
                        &[
                            Cols::Enabled as u32,
                            Cols::Location as u32,
                            Cols::File as u32,
                            Cols::Address as u32,
                            Cols::Cond as u32,
                            Cols::Hits as u32,
                        ],
                        &[
                            &mk_enabled_col(bp),
                            &mk_location_col(bp),
                            &mk_file_col(bp),
                            &mk_address_col(bp),
                            &mk_cond_col(bp),
                            &mk_hits_col(bp),
                        ],
                    );
                    return;
                }

                if !self.model.iter_next(&iter) {
                    break;
                }
            }
        }

        // Add a new row if we reach here
        let values: [&dyn gtk::ToValue; NUM_COLS] = [
            &mk_enabled_col(bp),
            &mk_number_col(bp),
            &mk_location_col(bp),
            &mk_file_col(bp),
            &mk_address_col(bp),
            &mk_cond_col(bp),
            &mk_hits_col(bp),
        ];
        self.model.set(&self.model.append(), &COL_INDICES, &values);
    }
}

fn mk_enabled_col(bp: &Breakpoint) -> gtk::Value {
    bp.enabled.to_value()
}

fn mk_number_col(bp: &Breakpoint) -> gtk::Value {
    format!("{}", bp.number).to_value()
}

fn mk_location_col(bp: &Breakpoint) -> gtk::Value {
    bp.original_location.to_value()
}

fn mk_file_col(bp: &Breakpoint) -> gtk::Value {
    match (&bp.file, &bp.line) {
        (Some(ref file), Some(ref line)) => format!("{}:{}", file, line).to_value(),
        _ => "".to_value(),
    }
}

fn mk_address_col(bp: &Breakpoint) -> gtk::Value {
    bp.address.to_value()
}

fn mk_cond_col(bp: &Breakpoint) -> gtk::Value {
    match bp.cond {
        None => "".to_value(),
        Some(ref cond) => cond.to_value(),
    }
}

fn mk_hits_col(bp: &Breakpoint) -> gtk::Value {
    format!("{}", bp.hits).to_value()
}
