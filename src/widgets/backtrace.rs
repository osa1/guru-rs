//! A `TreeView` for rendering backtraces.

use gtk::prelude::*;
use gio::prelude::*;

use crate::types::{Backtrace, Frame};

pub struct BacktraceW {
    model: gtk::ListStore,
    view: gtk::TreeView,
}

/// Column indices for cell renderers
#[repr(i32)]
enum BacktraceListCols {
    Level = 0,
    // e.g. "0x00000000007458e8"
    Addr,
    // e.g. "rtsFatalInternalErrorFn (s=0x7c34a5 "STABLE_NAME object (%p) entered!", ap=0x7ffe69a0c228)"
    Func,
    // e.g. "rts/RtsMessages.c:186"
    Loc,
}

/// Column types for the list store
static COL_TYPES: [gtk::Type; 4] = [
    gtk::Type::String, // level
    gtk::Type::String, // address
    gtk::Type::String, // function
    gtk::Type::String, // location
];

/// Column indices for when inserting rows into the list store
static COL_INDICES: [u32; 4] = [
    0, 1, 2, 3
];

impl BacktraceW {
    pub fn new(bt: &Backtrace) -> BacktraceW {

        //
        // Create the store (model)
        //

        let model = gtk::ListStore::new(&COL_TYPES);

        //
        // Create the view
        //

        let view = gtk::TreeView::new_with_model(&model);
        view.set_vexpand(false);
        view.set_headers_visible(false);

        let add_text_renderer_col =
            |title: &'static str, col_ty: BacktraceListCols, selectable: bool| {
                let renderer = gtk::CellRendererText::new();
                let column = gtk::TreeViewColumn::new();
                column.pack_start(&renderer, true);
                column.set_title(title);
                column.add_attribute(&renderer, "text", col_ty as i32);
                if selectable {
                    // We don't want to allow editing but we want to allow copying the contents, so we
                    // enable editing, but we don't update the text in "edited" callback.
                    renderer.set_property_editable(true);
                    renderer.connect_edited(|_w, _path, _str| {});
                }
                // Finally add the column
                view.append_column(&column);
            };

        add_text_renderer_col("Level", BacktraceListCols::Level, false);
        add_text_renderer_col("Address", BacktraceListCols::Addr, true);
        add_text_renderer_col("Function", BacktraceListCols::Func, true);
        add_text_renderer_col("Location", BacktraceListCols::Loc, true);

        //
        // Insert the rows
        //

        let ret = BacktraceW { model, view };
        ret.add_bt(bt);

        ret
    }

    /// Clear the contents (drop the frames). The widget will look like an empty list view.
    pub fn clear(&self) {
        self.model.clear();
    }

    /// Render the given backtrace. Note that old frames will be dropped.
    pub fn add_bt(&self, bt: &Backtrace) {
        self.clear();

        for frame in &bt.0 {
            let values: [&dyn gtk::ToValue; 4] = [
                &format!("#{}", frame.level),
                &frame.addr,
                &frame.func,
                &format!("{}:{}", frame.file, frame.line),
            ];

            self.model.set(&self.model.append(), &COL_INDICES, &values);
        }
    }
}
