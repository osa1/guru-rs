//! A `TreeView` for rendering breakpoints.

use gio::prelude::*;
use gtk::prelude::*;

use crate::types::{Breakpoint, BreakpointDisposition, BreakpointType};

pub struct BreakpointsW {
    model: gtk::ListStore,
    view: gtk::TreeView,
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
    pub fn new(breakpoints: &[Breakpoint]) -> BreakpointsW {
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
        let renderer = gtk::CellRendererToggle::new();
        renderer.connect_toggled(|_w, _path| { /* TODO */ });
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&renderer, true);
        column.set_title("Enabled");
        column.add_attribute(&renderer, "active", Cols::Enabled as i32);
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

        //
        // Insert the rows
        //

        let ret = BreakpointsW { view, model };
        for bp in breakpoints {
            ret.add_breakpoint(bp);
        }
        ret
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
