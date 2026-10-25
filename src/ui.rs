use crate::{
    config,
    server::{self, AppState},
};
use adw::prelude::*;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn error(parent: &adw::ApplicationWindow, text: &str) {
    let dialog = adw::MessageDialog::new(Some(parent), Some("Couldn't complete that"), Some(text));
    dialog.add_response("close", "Close");
    dialog.present();
}
fn save(state: &AppState, window: &adw::ApplicationWindow) {
    if let Err(e) = config::save(&state.lock().unwrap().config) {
        error(window, &e.to_string());
    }
}
pub fn run(state: AppState, port: u16, preview: bool, background: bool) {
    let app = adw::Application::builder()
        .application_id("io.github.glide.Touchpad")
        .build();
    let runtime: Rc<RefCell<Option<tokio::runtime::Runtime>>> = Rc::new(RefCell::new(None));
    let hold = Rc::new(RefCell::new(None));
    let runtime_start = runtime.clone();
    let state_start = state.clone();
    let hold_start = hold.clone();
    app.connect_startup(move |app| {
        *hold_start.borrow_mut() = Some(app.hold());
        let rt = tokio::runtime::Runtime::new().expect("Create runtime");
        let s = state_start.clone();
        rt.spawn(async move {
            if let Err(e) = server::run(s.clone(), port, preview).await {
                s.lock().unwrap().status = format!("Server unavailable: {e}");
            }
        });
        let (tray_tx, tray_rx) = std::sync::mpsc::channel();
        rt.spawn(crate::tray::run(tray_tx));
        let app_tray = app.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            while let Ok(action) = tray_rx.try_recv() {
                match action {
                    crate::tray::Action::Show => app_tray.activate(),
                    crate::tray::Action::Quit => {
                        app_tray.quit();
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });
        *runtime_start.borrow_mut() = Some(rt);
    });
    let first = Rc::new(RefCell::new(true));
    app.connect_activate(move |app| build_window(app, state.clone(), background, first.clone()));
    app.run_with_args::<&str>(&[]);
    drop(hold.borrow_mut().take());
    if let Some(rt) = runtime.borrow_mut().take() {
        rt.shutdown_timeout(Duration::from_secs(2));
    }
}

fn build_window(
    app: &adw::Application,
    state: AppState,
    background: bool,
    first: Rc<RefCell<bool>>,
) {
    if let Some(w) = app.active_window() {
        w.present();
        return;
    }
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Glide")
        .default_width(480)
        .default_height(720)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let menu = gio::Menu::new();
    menu.append(Some("Input Setup"), Some("app.setup"));
    menu.append(Some("About Glide"), Some("app.about"));
    menu.append(Some("Quit Glide"), Some("app.quit"));
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();
    header.pack_end(&menu_button);
    toolbar.add_top_bar(&header);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 24);
    body.set_margin_top(24);
    body.set_margin_bottom(28);
    body.set_margin_start(24);
    body.set_margin_end(24);
    let icon = gtk::Image::from_icon_name("input-touchpad-symbolic");
    icon.set_pixel_size(72);
    icon.add_css_class("accent");
    body.append(&icon);
    let title = gtk::Label::new(Some("Your phone. Your touchpad."));
    title.add_css_class("title-1");
    title.set_wrap(true);
    body.append(&title);
    let status = gtk::Label::new(Some("Starting…"));
    status.add_css_class("dim-label");
    status.set_wrap(true);
    body.append(&status);
    let connection = adw::PreferencesGroup::builder()
        .title("Connect a device")
        .description("Open this address on your phone, on the same Wi-Fi.")
        .build();
    let address = adw::ActionRow::builder()
        .title("Local address")
        .subtitle(&state.lock().unwrap().address)
        .build();
    let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy.set_tooltip_text(Some("Copy address"));
    copy.set_valign(gtk::Align::Center);
    address.add_suffix(&copy);
    let addr = state.lock().unwrap().address.clone();
    copy.connect_clicked(move |b| {
        b.clipboard().set_text(&addr);
    });
    connection.add(&address);
    let qr_button = gtk::Button::with_label("Show QR code");
    qr_button.set_valign(gtk::Align::Center);
    address.add_suffix(&qr_button);
    let qr_address = state.lock().unwrap().address.clone();
    let parent = window.clone();
    qr_button.connect_clicked(move |_| {
        if let Err(e) = show_qr(&parent, &qr_address) {
            error(&parent, &e.to_string());
        }
    });
    let pairing = adw::SwitchRow::builder()
        .title("Allow a new device")
        .subtitle("Enter the code below on your phone")
        .active(true)
        .build();
    connection.add(&pairing);
    let code = gtk::Label::new(Some(&state.lock().unwrap().pairing_code));
    code.add_css_class("title-1");
    code.add_css_class("monospace");
    code.set_margin_top(12);
    code.set_margin_bottom(12);
    connection.add(&code);
    body.append(&connection);
    let devices = adw::PreferencesGroup::builder()
        .title("Your devices")
        .build();
    let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list.add_css_class("boxed-list");
    devices.add(&list);
    body.append(&devices);
    let preferences = adw::PreferencesGroup::builder()
        .title("Preferences")
        .build();
    let enabled = adw::SwitchRow::builder()
        .title("Touchpad enabled")
        .subtitle("Turn off to pause remote input")
        .active(true)
        .build();
    preferences.add(&enabled);
    let bg = adw::SwitchRow::builder()
        .title("Run in background")
        .subtitle("Keep connected when this window closes")
        .active(state.lock().unwrap().config.background)
        .build();
    preferences.add(&bg);
    let startup = adw::SwitchRow::builder()
        .title("Start at login")
        .active(state.lock().unwrap().config.autostart)
        .build();
    preferences.add(&startup);
    body.append(&preferences);
    let note = gtk::Label::new(Some("Private to your network · No account needed"));
    note.add_css_class("dim-label");
    note.add_css_class("caption");
    body.append(&note);
    let clamp = adw::Clamp::builder().maximum_size(520).child(&body).build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build();
    toolbar.set_content(Some(&scroll));
    window.set_content(Some(&toolbar));
    let s = state.clone();
    pairing.connect_active_notify(move |r| {
        s.lock().unwrap().pairing = r.is_active();
    });
    let s = state.clone();
    enabled.connect_active_notify(move |r| {
        s.lock().unwrap().enabled = r.is_active();
    });
    let s = state.clone();
    let w = window.clone();
    bg.connect_active_notify(move |r| {
        s.lock().unwrap().config.background = r.is_active();
        save(&s, &w);
    });
    let s = state.clone();
    let w = window.clone();
    startup.connect_active_notify(move |r| {
        let enabled = r.is_active();
        if enabled == s.lock().unwrap().config.autostart {
            return;
        }
        match config::autostart(enabled) {
            Ok(()) => {
                s.lock().unwrap().config.autostart = enabled;
                save(&s, &w);
            }
            Err(e) => {
                r.set_active(!enabled);
                error(&w, &e.to_string());
            }
        }
    });
    let s = state.clone();
    let app_close = app.clone();
    window.connect_close_request(move |w| {
        if s.lock().unwrap().config.background {
            w.set_visible(false);
        } else {
            app_close.quit();
        }
        glib::Propagation::Stop
    });
    let quit = gio::SimpleAction::new("quit", None);
    let a = app.clone();
    quit.connect_activate(move |_, _| a.quit());
    app.add_action(&quit);
    let about = gio::SimpleAction::new("about", None);
    let w = window.clone();
    about.connect_activate(move |_, _| {
        adw::AboutWindow::builder()
            .transient_for(&w)
            .application_name("Glide")
            .application_icon("input-touchpad-symbolic")
            .version(env!("CARGO_PKG_VERSION"))
            .developer_name("Glide contributors")
            .comments("Turn a touchscreen into a native Linux touchpad.")
            .license_type(gtk::License::Gpl30)
            .build()
            .present();
    });
    app.add_action(&about);
    let setup = gio::SimpleAction::new("setup", None);
    let w = window.clone();
    setup.connect_activate(move|_,_|{
   let d=adw::MessageDialog::new(Some(&w),Some("Input setup"),Some("Glide uses /dev/uinput to create a virtual touchpad. It never needs to read your physical keyboard.\n\nIf input is unavailable, see the included setup guide for a dedicated-group udev rule. Only the one-time setup needs administrator access.\n\nThis first preview uses unencrypted HTTP on your trusted local network. Do not expose the port to the Internet."));d.add_response("close","Close");d.present();
  });
    app.add_action(&setup);
    let s = state.clone();
    let weak = window.downgrade();
    let mut last_devices = String::new();
    glib::timeout_add_local(Duration::from_millis(400), move || {
        if weak.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        let (summary, pairing_on, pin, devices_snapshot, current) = {
            let s = s.lock().unwrap();
            (
                if let Some((_, name)) = &s.connected {
                    format!("Connected to {name}")
                } else {
                    s.status.clone()
                },
                s.pairing,
                s.pairing_code.clone(),
                s.config.devices.clone(),
                s.connected.clone(),
            )
        };
        status.set_text(&summary);
        pairing.set_active(pairing_on);
        code.set_text(&pin);
        code.set_visible(pairing_on);
        let key = format!(
            "{}{:?}",
            serde_json::to_string(&devices_snapshot).unwrap(),
            current
        );
        if key != last_devices {
            last_devices = key;
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            if devices_snapshot.is_empty() {
                let empty = adw::ActionRow::builder()
                    .title("No paired devices yet")
                    .subtitle("Your phone will appear here after pairing")
                    .build();
                list.append(&empty);
            }
            for device in devices_snapshot {
                let connected = current.as_ref().is_some_and(|(id, _)| id == &device.id);
                let row = adw::ActionRow::builder()
                    .title(&device.name)
                    .subtitle(if connected {
                        "Connected"
                    } else {
                        "Ready to reconnect"
                    })
                    .build();
                let edit = gtk::Button::from_icon_name("emblem-system-symbolic");
                edit.set_tooltip_text(Some("Device settings"));
                edit.set_valign(gtk::Align::Center);
                row.add_suffix(&edit);
                let edit_state = s.clone();
                let edit_parent = weak.clone();
                let edit_id = device.id.clone();
                edit.connect_clicked(move |_| {
                    if let Some(parent) = edit_parent.upgrade() {
                        show_device_settings(&parent, edit_state.clone(), &edit_id);
                    }
                });
                let remove = gtk::Button::from_icon_name("user-trash-symbolic");
                remove.set_tooltip_text(Some("Forget device"));
                remove.set_valign(gtk::Align::Center);
                row.add_suffix(&remove);
                let st = s.clone();
                let win = weak.clone();
                remove.connect_clicked(move |_| {
                    let mut state = st.lock().unwrap();
                    let mut next = state.config.clone();
                    next.devices.retain(|d| d.id != device.id);
                    match config::save(&next) {
                        Ok(()) => state.config = next,
                        Err(e) => {
                            drop(state);
                            if let Some(w) = win.upgrade() {
                                error(&w, &e.to_string());
                            }
                        }
                    }
                });
                list.append(&row);
            }
        }
        glib::ControlFlow::Continue
    });
    if !background || !*first.borrow() {
        window.present();
    }
    *first.borrow_mut() = false;
}

fn show_qr(parent: &adw::ApplicationWindow, address: &str) -> anyhow::Result<()> {
    let (side, pixels) = crate::qr::pixels(address)?;
    let rgb: Vec<u8> = pixels.into_iter().flat_map(|v| [v, v, v]).collect();
    let texture = gtk::gdk::MemoryTexture::new(
        side as i32,
        side as i32,
        gtk::gdk::MemoryFormat::R8g8b8,
        &glib::Bytes::from_owned(rgb),
        side * 3,
    );
    let picture = gtk::Picture::for_paintable(&texture);
    picture.set_can_shrink(true);
    picture.set_size_request(264, 264);
    picture.update_property(&[gtk::accessible::Property::Label(
        "QR code for the Glide connection address",
    )]);
    let dialog = adw::Window::builder()
        .title("Scan to connect")
        .transient_for(parent)
        .modal(true)
        .default_width(380)
        .resizable(false)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.append(&picture);
    let label = gtk::Label::new(Some(address));
    label.set_selectable(true);
    content.append(&label);
    let hint = gtk::Label::new(Some(
        "Scan with your phone’s camera, then enter the pairing code shown in Glide.",
    ));
    hint.set_wrap(true);
    hint.set_max_width_chars(36);
    hint.set_justify(gtk::Justification::Center);
    hint.add_css_class("dim-label");
    content.append(&hint);
    toolbar.set_content(Some(&content));
    dialog.set_content(Some(&toolbar));
    dialog.present();
    Ok(())
}

fn show_device_settings(parent: &adw::ApplicationWindow, state: AppState, id: &str) {
    let Some(device) = state
        .lock()
        .unwrap()
        .config
        .devices
        .iter()
        .find(|d| d.id == id)
        .cloned()
    else {
        return;
    };
    let dialog = adw::Window::builder()
        .title(format!("{} — Settings", device.name))
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let group = adw::PreferencesGroup::builder()
        .title("Device settings")
        .description("Saved on this desktop. Changes also appear on the connected device.")
        .build();
    group.set_margin_top(24);
    group.set_margin_bottom(24);
    group.set_margin_start(24);
    group.set_margin_end(24);
    let cursor = gtk::SpinButton::with_range(0.25, 2.0, 0.05);
    cursor.set_digits(2);
    cursor.set_value(device.settings.cursor_speed);
    cursor.set_valign(gtk::Align::Center);
    let scroll = gtk::SpinButton::with_range(0.25, 2.0, 0.05);
    scroll.set_digits(2);
    scroll.set_value(device.settings.scroll_speed);
    scroll.set_valign(gtk::Align::Center);
    for (title, spin) in [("Cursor speed", &cursor), ("Scroll speed", &scroll)] {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle("Multiplier · 1.00 is the original feel")
            .build();
        row.add_suffix(spin);
        row.set_activatable_widget(Some(spin));
        spin.update_property(&[gtk::accessible::Property::Label(title)]);
        group.add(&row);
    }
    let dots = adw::SwitchRow::builder()
        .title("Show touch points")
        .active(device.settings.show_points)
        .build();
    group.add(&dots);
    let media_controls = adw::SwitchRow::builder()
        .title("Four-finger media controls")
        .subtitle("Tap: play/pause · up/down: volume · left/right: previous/next")
        .active(device.settings.media_controls)
        .build();
    group.add(&media_controls);
    let haptics = adw::SwitchRow::builder()
        .title("Tap feedback")
        .subtitle("Brief vibration on supported phones and browsers")
        .active(device.settings.haptics)
        .build();
    group.add(&haptics);
    let pulse = gtk::SpinButton::with_range(5.0, 100.0, 1.0);
    pulse.set_value(f64::from(device.settings.haptic_duration_ms));
    pulse.set_valign(gtk::Align::Center);
    pulse.update_property(&[gtk::accessible::Property::Label(
        "Pulse duration in milliseconds",
    )]);
    let pulse_row = adw::ActionRow::builder()
        .title("Pulse duration")
        .subtitle("5–100 milliseconds · shorter pulses feel crisper")
        .build();
    pulse_row.add_suffix(&pulse);
    pulse_row.set_activatable_widget(Some(&pulse));
    group.add(&pulse_row);
    let theme = gtk::DropDown::from_strings(&["System", "Dark", "Light"]);
    theme.set_selected(match device.settings.theme.as_str() {
        "dark" => 1,
        "light" => 2,
        _ => 0,
    });
    theme.set_valign(gtk::Align::Center);
    let row = adw::ActionRow::builder().title("Appearance").build();
    row.add_suffix(&theme);
    theme.update_property(&[gtk::accessible::Property::Label("Appearance")]);
    group.add(&row);
    let message = gtk::Label::new(Some("Saved on your desktop"));
    message.set_wrap(true);
    message.set_margin_top(16);
    message.add_css_class("dim-label");
    group.add(&message);
    let content = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&group)
        .build();
    toolbar.set_content(Some(&content));
    dialog.set_content(Some(&toolbar));
    let syncing = Rc::new(Cell::new(false));
    let st = state.clone();
    let device_id = id.to_owned();
    let guard = syncing.clone();
    let c = cursor.clone();
    let sc = scroll.clone();
    let d = dots.clone();
    let h = haptics.clone();
    let media = media_controls.clone();
    let duration = pulse.clone();
    let t = theme.clone();
    let msg = message.clone();
    let persist: Rc<dyn Fn()> = Rc::new(move || {
        if guard.get() {
            return;
        }
        let settings = config::DeviceSettings {
            cursor_speed: c.value(),
            scroll_speed: sc.value(),
            show_points: d.is_active(),
            haptics: h.is_active(),
            media_controls: media.is_active(),
            haptic_duration_ms: duration.value_as_int() as u16,
            theme: ["system", "dark", "light"][t.selected() as usize].into(),
        };
        match server::save_settings(&mut st.lock().unwrap(), &device_id, settings) {
            Ok(()) => msg.set_text("Saved on your desktop"),
            Err(e) => msg.set_text(&format!("Could not save: {e}")),
        }
    });
    let cb = persist.clone();
    cursor.connect_value_changed(move |_| cb());
    let cb = persist.clone();
    scroll.connect_value_changed(move |_| cb());
    let cb = persist.clone();
    dots.connect_active_notify(move |_| cb());
    let cb = persist.clone();
    haptics.connect_active_notify(move |_| cb());
    let cb = persist.clone();
    media_controls.connect_active_notify(move |_| cb());
    let cb = persist.clone();
    pulse.connect_value_changed(move |_| cb());
    theme.connect_selected_notify(move |_| persist());
    let weak = dialog.downgrade();
    let device_id = id.to_owned();
    glib::timeout_add_local(Duration::from_millis(400), move || {
        let Some(window) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let settings = state
            .lock()
            .unwrap()
            .config
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .map(|d| d.settings.clone());
        let Some(settings) = settings else {
            window.close();
            return glib::ControlFlow::Break;
        };
        syncing.set(true);
        if cursor.value() != settings.cursor_speed {
            cursor.set_value(settings.cursor_speed);
        }
        if scroll.value() != settings.scroll_speed {
            scroll.set_value(settings.scroll_speed);
        }
        dots.set_active(settings.show_points);
        haptics.set_active(settings.haptics);
        media_controls.set_active(settings.media_controls);
        if pulse.value_as_int() != i32::from(settings.haptic_duration_ms) {
            pulse.set_value(f64::from(settings.haptic_duration_ms));
        }
        theme.set_selected(match settings.theme.as_str() {
            "dark" => 1,
            "light" => 2,
            _ => 0,
        });
        syncing.set(false);
        glib::ControlFlow::Continue
    });
    dialog.present();
}
