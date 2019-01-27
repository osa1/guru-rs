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

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
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
}
