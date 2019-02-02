//! A `TreeView` for rendering breakpoints.

use gio::prelude::*;
use gtk::prelude::*;

use crate::types::{Breakpoint, BreakpointDisposition, BreakpointType};

pub struct BreakpointsW {
    model: gtk::ListStore,
    view: gtk::TreeView,
    bp_enabled_renderer: gtk::CellRendererToggle,
}

// TODO: How to best show disposition?

/// Number of columns
const NUM_COLS: usize = 6;

/// Column indices for cell renderers
#[repr(i32)]
enum Cols {
    Enabled = 0,
    Number,
    // E.g. foo.c:123
    Location,
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
    gtk::Type::String, // address
    gtk::Type::String, // condition
    gtk::Type::String, // hits
];

/// Column indices for when inserting rows into the list store
static COL_INDICES: [u32; NUM_COLS] = [0, 1, 2, 3, 4, 5];

impl BreakpointsW {
    pub fn new() -> BreakpointsW {
        //
        // Create the store (model)
        //

        let model = gtk::ListStore::new(&COL_TYPES);

        //
        // Create the view
        //

        let view = gtk::TreeView::new_with_model(&model);
        view.set_vexpand(false);
        view.set_hexpand(false);
        view.set_headers_visible(true);

        // Enabled column, render as a toggle
        let bp_enabled_renderer = gtk::CellRendererToggle::new();
        // bp_enabled.connect_toggled(|_w, _path| { /* TODO */ });
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
        add_col("Address", Cols::Address, false);
        add_col("Condition", Cols::Cond, true);
        add_col("Hits", Cols::Hits, false);

        BreakpointsW {
            view,
            model,
            bp_enabled_renderer,
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
        self.view.upcast_ref()
    }

    pub fn add_breakpoint(&self, bp: &Breakpoint) {
        let values: [&dyn gtk::ToValue; NUM_COLS] = [
            &bp.enabled,
            &format!("{}", bp.number),
            &format!("{}:{}", bp.file, bp.line),
            &bp.address,
            match bp.cond {
                None => &"",
                Some(ref cond) => cond,
            },
            &format!("{}", bp.hits),
        ];
        self.model.set(&self.model.append(), &COL_INDICES, &values);
    }
}
