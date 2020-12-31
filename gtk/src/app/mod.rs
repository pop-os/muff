pub mod events;
pub mod signals;
pub mod state;
pub mod views;
pub mod widgets;

use self::events::*;
use self::state::*;
use self::views::*;
use self::widgets::*;

use gtk::{self, prelude::*};
use std::{fs::File, process, rc::Rc, sync::Arc};

const CSS: &str = include_str!("ui.css");

pub struct App {
    pub ui: Rc<GtkUi>,
    pub state: Arc<State>,
}

impl App {
    pub fn new(state: State) -> Self {
        if gtk::init().is_err() {
            eprintln!("failed to initialize GTK Application");
            process::exit(1);
        }

        App { ui: Rc::new(GtkUi::new()), state: Arc::new(state) }
    }

    pub fn connect_events(self) -> Self {
        self.connect_back();
        self.connect_next();
        self.connect_ui_events();
        self.connect_image_chooser();
        self.connect_image_drag_and_drop();
        self.connect_hash();
        self.connect_view_ready();
        self.connect_check();

        self
    }

    pub fn then_execute(self) {
        self.ui.window.show_all();
        gtk::main();
    }
}

pub struct GtkUi {
    window: gtk::Window,
    header: Header,
    content: Content,
}

impl GtkUi {
    pub fn new() -> Self {
        // Create a the headerbar and it's associated content.
        let header = Header::new();
        let content = Content::new();

        // Create a new top level window.
        let window = cascade! {
            gtk::Window::new(gtk::WindowType::Toplevel);
            // Set the headerbar as the title bar widget.
            ..set_titlebar(Some(&header.container));
            // Set the title of the window.
            ..set_title("Popsicle");
            // The default size of the window to create.
            ..set_default_size(500, 250);
            ..add(&content.container);
        };

        // Add a custom CSS style
        let screen = window.get_screen().unwrap();
        let style = gtk::CssProvider::new();
        let _ = gtk::CssProviderExt::load_from_data(&style, CSS.as_bytes());
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &style,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );

        // The icon the app will display.
        gtk::Window::set_default_icon_name("com.system76.Popsicle");

        // Programs what to do when the exit button is used.
        window.connect_delete_event(move |_, _| {
            gtk::main_quit();
            gtk::Inhibit(false)
        });

        GtkUi { header, window, content }
    }

    pub fn errorck<T, E: ::std::fmt::Display>(
        &self,
        state: &State,
        result: Result<T, E>,
        context: &'static str,
    ) -> Result<T, ()> {
        result.map_err(|why| {
            self.content.error_view.view.description.set_text(&format!("{}: {}", context, why));
            self.switch_to(state, ActiveView::Error);
        })
    }

    pub fn errorck_option<T>(
        &self,
        state: &State,
        result: Option<T>,
        context: &'static str,
    ) -> Result<T, ()> {
        result.ok_or_else(|| {
            self.content
                .error_view
                .view
                .description
                .set_text(&format!("{}: no value found", context));
            self.switch_to(state, ActiveView::Error);
        })
    }

    pub fn switch_to(&self, state: &State, view: ActiveView) {
        let back = &self.header.back;
        let next = &self.header.next;
        let stack = &self.content.container;

        let back_ctx = back.get_style_context();
        let next_ctx = next.get_style_context();

        let widget = match view {
            ActiveView::Images => {
                back.set_label("Cancel");
                back_ctx.remove_class("back-button");
                back_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);

                next.set_label("Next");
                next.set_visible(true);
                next.set_sensitive(true);
                next_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                next_ctx.add_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);

                &self.content.image_view.view.container
            }
            ActiveView::Devices => {
                next_ctx.remove_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);
                next_ctx.add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                next.set_sensitive(false);

                let _ = state.back_event_tx.send(BackgroundEvent::RefreshDevices);
                &self.content.devices_view.view.container
            }
            ActiveView::Flashing => {
                match self.errorck(
                    &state,
                    File::open(&*state.image_path.borrow()),
                    "Failed to open ISO",
                ) {
                    Ok(file) => *state.image.borrow_mut() = Some(file),
                    Err(()) => return,
                };

                let all_devices = state.available_devices.borrow();
                let mut devices = state.selected_devices.borrow_mut();

                devices.clear();

                for active_id in self.content.devices_view.get_active_ids() {
                    devices.push(all_devices[active_id].clone());
                }

                back_ctx.remove_class("back-button");
                back_ctx.add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);

                next.set_visible(false);
                &self.content.flash_view.view.container
            }
            ActiveView::Summary => {
                back_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                back.set_label("Flash Again");

                next_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                next.set_visible(true);
                next.set_label("Done");
                &self.content.summary_view.view.container
            }
            ActiveView::Error => {
                back.set_label("Flash Again");
                back_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);

                next.set_visible(true);
                next.set_label("Close");
                next_ctx.remove_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
                next_ctx.remove_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);

                &self.content.error_view.view.container
            }
        };

        stack.set_visible_child(widget);
        state.active_view.set(view);
    }
}
