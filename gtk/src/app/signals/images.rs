use crate::app::events::{BackgroundEvent, UiEvent};
use crate::app::state::State;
use crate::app::widgets::OpenDialog;
use crate::app::{App, GtkUi};
use crate::misc;
use gtk::prelude::*;
use std::path::{Path, PathBuf};

impl App {
    pub fn connect_image_chooser(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        self.ui.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                let _ = state.ui_event_tx.send(UiEvent::SetImageLabel(path));
                set_hash_widget(&state, &ui);
            }
        });
    }

    pub fn connect_hash(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        self.ui.content.image_view.check.connect_clicked(move |_| {
            set_hash_widget(&state, &ui);
        });
    }

    pub fn connect_image_drag_and_drop(&self) {
        let state = self.state.clone();
        let ui = self.ui.clone();
        let image_view = ui.content.image_view.view.container.clone();

        misc::drag_and_drop(&image_view, move |data| {
            if let Some(uri) = data.get_text() {
                if uri.starts_with("file://") {
                    let path = Path::new(&uri[7..uri.len() - 1]);
                    if path.extension().map_or(false, |ext| ext == "iso" || ext == "img")
                        && path.exists()
                    {
                        let _ = state.ui_event_tx.send(UiEvent::SetImageLabel(path.to_path_buf()));
                        set_hash_widget(&state, &ui);
                    }
                }
            }
        });
    }

    pub fn connect_check(&self) {
        let state = self.state.clone();
        self.ui.content.image_view.check_button.connect_property_active_notify(move |button| {
            state.check.set(button.get_active());
        });
    }
}

fn set_hash_widget(state: &State, ui: &GtkUi) {
    let hash = &ui.content.image_view.hash;

    let path = state.image_path.borrow();
    let kind = match hash.get_active() {
        Some(1) => "SHA256",
        Some(2) => "MD5",
        _ => return,
    };

    ui.content.image_view.chooser_container.set_visible_child_name("checksum");
    ui.content.image_view.set_hash_sensitive(false);

    let _ = state.back_event_tx.send(BackgroundEvent::GenerateHash(PathBuf::from(&*path), kind));
}
