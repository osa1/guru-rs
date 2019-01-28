extern crate cairo;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;

mod gdb;
mod mi;
mod parsers;
mod types;
mod widgets;

use gio::prelude::*;
use gtk::prelude::*;

fn main() {
    let application =
        gtk::Application::new(None, Default::default()).expect("Initialization failed...");

    application.connect_startup(build_ui);
    application.connect_activate(|_| {});

    application.run(&[]);
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_default_size(500, 850);
    window.set_title("guru");

    // Horizontal: | Vertical: -

    // horiz(1) -> [ vert(1) -> [ vert(2) -> [ currently_empty, gdb logs ], breakpoints ], threads ]
    let horiz1 = gtk::Paned::new(gtk::Orientation::Horizontal);
    let vert1 = gtk::Paned::new(gtk::Orientation::Vertical);
    horiz1.add1(&vert1);
    window.add(&horiz1);

    let mut bts = vec![];
    let frame_strs = vec![
        FRAME_0, FRAME_1, FRAME_2, FRAME_3, FRAME_4, FRAME_5, FRAME_6, FRAME_7,
    ];
    for frame_str in frame_strs {
        let bt_mi = mi::parser::parse_value(frame_str).unwrap().0;
        let result_list = bt_mi.get_result_list().unwrap();
        println!("bt_mi result list: {:?}", result_list);
        let bt = parsers::parse_backtrace(result_list).unwrap();
        bts.push(bt);
    }

    let threads_widget = widgets::ThreadsW::new(&bts);
    horiz1.add2(threads_widget.get_widget());

    // Breakpoints
    let mut bps = vec![];
    let mut bp_strings = vec![BP_1, BP_2];
    for bp_string in bp_strings {
        let bp_mi = mi::parser::parse_value(bp_string).unwrap().0;
        let bp_tuple = bp_mi.get_tuple().unwrap();
        println!("bp_mi tuple: {:?}", bp_tuple);
        let bp = parsers::parse_breakpoint(bp_tuple).unwrap();
        bps.push(bp);
    }
    let bps = widgets::BreakpointsW::new(&bps);

    let vert2 = gtk::Paned::new(gtk::Orientation::Vertical);
    vert1.pack1(&vert2, true, false);
    vert1.pack2(bps.get_widget(), true, true);
    vert1.set_position(100);

    let gdb_view = widgets::GdbW::new();
    let some_label = gtk::Label::new("foo");
    vert2.pack2(gdb_view.get_widget(), true, false);
    vert2.set_position(100);
    // vert1.add1(gdb_view.get_widget());

    window.show_all();

    // This only works after rendering
    threads_widget.reset_cols();
}

static FRAME_0: &'static str = "[frame={level=\"0\",addr=\"0x00000000006eff82\",func=\"initCapabilities\",file=\"rts/Capability.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/Capability.c\",line=\"398\"},frame={level=\"1\",addr=\"0x00000000006ee476\",func=\"initScheduler\",file=\"rts/Schedule.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/Schedule.c\",line=\"2680\"},frame={level=\"2\",addr=\"0x00000000006e8cc0\",func=\"hs_init_ghc\",file=\"rts/RtsStartup.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/RtsStartup.c\",line=\"236\"},frame={level=\"3\",addr=\"0x0000000000701f08\",func=\"hs_main\",file=\"rts/RtsMain.c\",fullname=\"/home/omer/haskell/ghc-gc/rts/RtsMain.c\",line=\"57\"},frame={level=\"4\",addr=\"0x0000000000405366\",func=\"main\"}]";

static FRAME_1: &'static str = "[frame={level=\"0\",addr=\"0x00007ffff6f8d9f3\",func=\"futex_wait_cancelable\",file=\"../sysdeps/unix/sysv/linux/futex-internal.h\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/../sysdeps/unix/sysv/linux/futex-internal.h\",line=\"88\"},frame={level=\"1\",addr=\"0x00007ffff6f8d9f3\",func=\"__pthread_cond_wait_common\",file=\"pthread_cond_wait.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/pthread_cond_wait.c\",line=\"502\"},frame={level=\"2\",addr=\"0x00007ffff6f8d9f3\",func=\"__pthread_cond_wait\",file=\"pthread_cond_wait.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/pthread_cond_wait.c\",line=\"655\"},frame={level=\"3\",addr=\"0x00000000004b883f\",func=\"waitCondition\",file=\"rts/posix/OSThreads.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/posix/OSThreads.c\",line=\"117\"},frame={level=\"4\",addr=\"0x00000000004a6bff\",func=\"waitForWorkerCapability\",file=\"rts/Capability.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Capability.c\",line=\"651\"},frame={level=\"5\",addr=\"0x00000000004a6bff\",func=\"yieldCapability\",file=\"rts/Capability.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Capability.c\",line=\"888\"},frame={level=\"6\",addr=\"0x00000000004a47c9\",func=\"scheduleYield\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"672\"},frame={level=\"7\",addr=\"0x00000000004a47c9\",func=\"schedule\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"292\"},frame={level=\"8\",addr=\"0x00000000004a5489\",func=\"scheduleWaitThread\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"2533\"},frame={level=\"9\",addr=\"0x00000000004a5bc1\",func=\"rts_evalLazyIO\",file=\"rts/RtsAPI.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/RtsAPI.c\",line=\"530\"},frame={level=\"10\",addr=\"0x00000000004afc07\",func=\"hs_main\",file=\"rts/RtsMain.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/RtsMain.c\",line=\"72\"},frame={level=\"11\",addr=\"0x000000000042fb3e\",func=\"main\"}]";

static FRAME_2: &'static str = "[frame={level=\"0\",addr=\"0x00007ffff684abb7\",func=\"epoll_wait\",file=\"../sysdeps/unix/sysv/linux/epoll_wait.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/epoll_wait.c\",line=\"30\"},frame={level=\"1\",addr=\"0x00000000004839af\",func=\"base_GHCziEventziEPoll_new10_info\"},frame={level=\"2\",addr=\"0x0000000000000000\",func=\"??\"}]";

static FRAME_3: &'static str = "[frame={level=\"0\",addr=\"0x00007ffff6f91384\",func=\"__libc_read\",file=\"../sysdeps/unix/sysv/linux/read.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/../sysdeps/unix/sysv/linux/read.c\",line=\"27\"},frame={level=\"1\",addr=\"0x00000000004b8aba\",func=\"itimer_thread_func\",file=\"rts/posix/itimer/Pthread.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/posix/itimer/Pthread.c\",line=\"124\"},frame={level=\"2\",addr=\"0x00007ffff6f876db\",func=\"start_thread\",file=\"pthread_create.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/pthread_create.c\",line=\"463\"},frame={level=\"3\",addr=\"0x00007ffff684a88f\",func=\"clone\",file=\"../sysdeps/unix/sysv/linux/x86_64/clone.S\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/clone.S\",line=\"95\"}]";

static FRAME_4: &'static str = "[frame={level=\"0\",addr=\"0x00007ffff684abb7\",func=\"epoll_wait\",file=\"../sysdeps/unix/sysv/linux/epoll_wait.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/epoll_wait.c\",line=\"30\"},frame={level=\"1\",addr=\"0x00000000004839af\",func=\"base_GHCziEventziEPoll_new10_info\"},frame={level=\"2\",addr=\"0x0000000000000000\",func=\"??\"}]";

static FRAME_5: &'static str = "[frame={level=\"0\",addr=\"0x00007ffff684abb7\",func=\"epoll_wait\",file=\"../sysdeps/unix/sysv/linux/epoll_wait.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/epoll_wait.c\",line=\"30\"},frame={level=\"1\",addr=\"0x00000000004839af\",func=\"base_GHCziEventziEPoll_new10_info\"},frame={level=\"2\",addr=\"0x0000000000000000\",func=\"??\"}]";

static FRAME_6: &'static str = "[frame={level=\"0\",addr=\"0x00000000004b2281\",func=\"GarbageCollect\",file=\"rts/sm/GC.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/sm/GC.c\",line=\"203\"},frame={level=\"1\",addr=\"0x00000000004a3e8d\",func=\"scheduleDoGC\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"1797\"},frame={level=\"2\",addr=\"0x00000000004a4d63\",func=\"schedule\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"545\"},frame={level=\"3\",addr=\"0x00000000004a54b1\",func=\"scheduleWorker\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"2550\"},frame={level=\"4\",addr=\"0x00000000004ac12a\",func=\"workerStart\",file=\"rts/Task.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Task.c\",line=\"444\"},frame={level=\"5\",addr=\"0x00007ffff6f876db\",func=\"start_thread\",file=\"pthread_create.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/pthread_create.c\",line=\"463\"},frame={level=\"6\",addr=\"0x00007ffff684a88f\",func=\"clone\",file=\"../sysdeps/unix/sysv/linux/x86_64/clone.S\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/clone.S\",line=\"95\"}]";

static FRAME_7: &'static str = "[frame={level=\"0\",addr=\"0x00000000004b3db1\",func=\"ACQUIRE_SPIN_LOCK\",file=\"includes/rts/SpinLock.h\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/includes/rts/SpinLock.h\",line=\"47\"},frame={level=\"1\",addr=\"0x00000000004b3db1\",func=\"gcWorkerThread\",file=\"rts/sm/GC.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/sm/GC.c\",line=\"1143\"},frame={level=\"2\",addr=\"0x00000000004a6b48\",func=\"yieldCapability\",file=\"rts/Capability.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Capability.c\",line=\"861\"},frame={level=\"3\",addr=\"0x00000000004a47c9\",func=\"scheduleYield\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"672\"},frame={level=\"4\",addr=\"0x00000000004a47c9\",func=\"schedule\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"292\"},frame={level=\"5\",addr=\"0x00000000004a54b1\",func=\"scheduleWorker\",file=\"rts/Schedule.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Schedule.c\",line=\"2550\"},frame={level=\"6\",addr=\"0x00000000004ac12a\",func=\"workerStart\",file=\"rts/Task.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/Task.c\",line=\"444\"},frame={level=\"7\",addr=\"0x00007ffff6f876db\",func=\"start_thread\",file=\"pthread_create.c\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/nptl/pthread_create.c\",line=\"463\"},frame={level=\"8\",addr=\"0x00007ffff684a88f\",func=\"clone\",file=\"../sysdeps/unix/sysv/linux/x86_64/clone.S\",fullname=\"/build/glibc-OTsEL5/glibc-2.27/misc/../sysdeps/unix/sysv/linux/x86_64/clone.S\",line=\"95\"}]";

static BP_1: &'static str = "{number=\"1\",type=\"breakpoint\",disp=\"keep\",enabled=\"y\",addr=\"0x00000000004b2281\",func=\"GarbageCollect\",file=\"rts/sm/GC.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/sm/GC.c\",line=\"203\",thread-groups=[\"i1\"],times=\"1\",original-location=\"GarbageCollect\"}";

static BP_2: &'static str = "{number=\"2\",type=\"breakpoint\",disp=\"keep\",enabled=\"y\",addr=\"0x00000000004ccb0d\",func=\"evacuate\",file=\"rts/sm/Evac.c\",fullname=\"/home/ben/bin-dist-8.6.3-Linux-dwarf/ghc/rts/sm/Evac.c\",line=\"502\",thread-groups=[\"i1\"],cond=\"p == 0x0\",times=\"0\",original-location=\"evacuate\"}";
