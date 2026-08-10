use adw::prelude::*;
use gtk::gio;

use crate::{config, i18n::gettext, window};

pub fn run() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(config::app_id())
        .build();

    install_actions(&app);
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }

        let window = window::build_window(app);
        #[cfg(debug_assertions)]
        crate::icons::audit_runtime();
        window.present();
    });

    app.run()
}

fn install_actions(app: &adw::Application) {
    let quit = gio::SimpleAction::new("quit", None);
    let app_weak = app.downgrade();
    quit.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<primary>q"]);

    let about = gio::SimpleAction::new("about", None);
    let app_weak = app.downgrade();
    about.connect_activate(move |_, _| {
        let Some(app) = app_weak.upgrade() else {
            return;
        };
        let dialog = adw::AboutDialog::builder()
            .application_name("Data Inspector")
            .application_icon(config::app_id())
            .developer_name("Christiaan Bruinsma")
            .version(config::VERSION)
            .license_type(gtk::License::Gpl30)
            .comments(gettext(
                "Inspect structured data files locally in a clear, readable view.",
            ))
            .build();

        if let Some(window) = app.active_window() {
            dialog.present(Some(&window));
        } else {
            dialog.present(None::<&gtk::Widget>);
        }
    });
    app.add_action(&about);
}
