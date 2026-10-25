use ksni::{Tray, TrayMethods, menu::StandardItem};
use std::sync::mpsc::Sender;

pub enum Action {
    Show,
    Quit,
}
struct GlideTray {
    actions: Sender<Action>,
}
impl Tray for GlideTray {
    fn id(&self) -> String {
        "glide".into()
    }
    fn title(&self) -> String {
        "Glide".into()
    }
    fn icon_name(&self) -> String {
        if std::env::var_os("APPIMAGE").is_some() {
            "io.github.glide.Touchpad".into()
        } else {
            "input-touchpad-symbolic".into()
        }
    }
    fn icon_theme_path(&self) -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent()?.parent().map(|p| p.join("share/icons")))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.actions.send(Action::Show);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open Glide".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.actions.send(Action::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit Glide".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.actions.send(Action::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        true
    }
}
pub async fn run(actions: Sender<Action>) {
    // Login autostart may precede GNOME's tray extension. Keep waiting/re-register
    // when the watcher appears; never make touchpad startup depend on the tray.
    match (GlideTray { actions })
        .assume_sni_available(true)
        .spawn()
        .await
    {
        Ok(_handle) => std::future::pending::<()>().await,
        Err(error) => eprintln!("Tray unavailable: {error}"),
    }
}
