//! A scrolled widget that shows thread backtraces.

use gio::prelude::*;
use gtk::prelude::*;

use crate::types::Backtrace;
use crate::widgets::backtrace::BacktraceW;

// TODO: Make the threads draggable. We should remember the positions when updating. (so if I
// re-order threads to 1-3-2, after updating this widget with new backtraces I should still get
// 1-3-2)

pub struct ThreadsW {
    // scrolled -> box -> [expander -> BacktraceW]
    widget: gtk::ScrolledWindow,
    box_: gtk::Box,
    threads: Vec<BacktraceW>,
}

impl ThreadsW {
    pub fn new() -> ThreadsW {
        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_.set_baseline_position(gtk::BaselinePosition::Top);
        scrolled.add(&box_);

        ThreadsW {
            widget: scrolled,
            box_,
            threads: vec![],
        }
    }

    /// ONLY USE TO ADD THIS TO CONTAINERS!
    pub fn get_widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }

    pub fn clear(&mut self) {
        for thread in &self.threads {
            self.box_.remove(thread.get_widget());
        }
        self.threads.clear();
    }

    pub fn add_thread(&mut self, thread_id: i32, target_id: &str, bt: &Backtrace) {
        let expander = gtk::Expander::new(Some(format!("#{} {}", thread_id, target_id).as_str()));
        expander.set_expanded(true);
        expander.set_vexpand(false);
        let w = BacktraceW::new(bt);
        expander.add(w.get_widget());
        self.box_.pack_start(&expander, false, false, 0);
        self.threads.push(w);
        self.box_.show_all();
        // This traversel all rows in every addition but OK
        // self.reset_cols();
    }

    /// Make same columns of different thread views the same. Note that this only works after
    /// rendering the widget (e.g. after show_all()).
    pub fn reset_cols(&self) {
        let mut max_1 = 0;
        let mut max_2 = 0;
        let mut max_3 = 0;
        let mut max_4 = 0;
        for t in &self.threads {
            let (c1, c2, c3, c4) = t.get_col_widths();
            if c1 > max_1 {
                max_1 = c1;
            }
            if c2 > max_2 {
                max_2 = c2;
            }
            if c3 > max_3 {
                max_3 = c3;
            }
            if c4 > max_4 {
                max_4 = c4;
            }
        }
        println!("{} {} {} {}", max_1, max_2, max_3, max_4);
        for t in &self.threads {
            t.set_col_widths(max_1, max_2, max_3, max_4);
        }
    }
}
