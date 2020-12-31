use crate::app::events::{self, BackgroundEvent, UiEvent};
use atomic::Atomic;
use crossbeam_channel::{unbounded, Receiver, Sender};
use dbus_udisks2::DiskDevice;
use libc;
use std::cell::{Cell, RefCell};
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveView {
    Images,
    Devices,
    Flashing,
    Summary,
    Error,
}

pub struct State {
    pub ui_event_tx: Sender<UiEvent>,
    pub ui_event_rx: Receiver<UiEvent>,
    pub back_event_tx: Sender<BackgroundEvent>,

    pub active_view: Cell<ActiveView>,

    pub image: RefCell<Option<File>>,
    pub image_path: RefCell<PathBuf>,
    pub image_size: Arc<Atomic<u64>>,

    pub available_devices: RefCell<Box<[Arc<DiskDevice>]>>,
    pub selected_devices: RefCell<Vec<Arc<DiskDevice>>>,

    pub check: Cell<bool>,
}

impl State {
    pub fn new() -> Self {
        let (back_event_tx, back_event_rx) = unbounded();
        let (ui_event_tx, ui_event_rx) = unbounded();

        // If running in pkexec or sudo, restore home directory for open dialog,
        // and then downgrade permissions back to a regular user.
        if let Ok(pkexec_uid) = env::var("PKEXEC_UID").or_else(|_| env::var("SUDO_UID")) {
            if let Ok(uid) = pkexec_uid.parse::<u32>() {
                if let Some(passwd) = pwd::Passwd::from_uid(uid) {
                    env::set_var("HOME", passwd.dir);
                    downgrade_permissions(passwd.uid, passwd.gid);
                }
            }
        }

        events::background_thread(ui_event_tx.clone(), back_event_rx);

        Self {
            ui_event_rx,
            ui_event_tx,
            back_event_tx,
            active_view: Cell::new(ActiveView::Images),
            image: RefCell::new(None),
            image_path: RefCell::new(PathBuf::new()),
            image_size: Arc::new(Atomic::new(0u64)),
            available_devices: RefCell::new(Box::new([])),
            selected_devices: RefCell::new(Vec::new()),
            check: Cell::new(false),
        }
    }
}

/// Downgrades the permissions of the current thread to the specified user and group ID.
fn downgrade_permissions(uid: u32, gid: u32) {
    unsafe {
        libc::setresgid(gid, gid, gid);
        libc::setresuid(uid, uid, uid);
    }
}
