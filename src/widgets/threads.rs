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
    threads: Vec<BacktraceW>,
}

impl ThreadsW {
    pub fn new(threads: &[Backtrace]) -> ThreadsW {
        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_.set_baseline_position(gtk::BaselinePosition::Top);
        scrolled.add(&box_);

        let mut ws = Vec::with_capacity(threads.len());
        for (thread_idx, thread) in threads.iter().enumerate() {
            let expander = gtk::Expander::new(Some(format!("Thread {}", thread_idx).as_str()));
            expander.set_expanded(true);
            let w = BacktraceW::new(thread);
            expander.add(w.get_widget());
            box_.pack_start(&expander, false, false, 0);
            ws.push(w);
        }

        ThreadsW {
            widget: scrolled,
            threads: ws,
        }
    }

    /// ONLY USE TO ADD THIS TO CONTAINERS!
    pub fn get_widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
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
