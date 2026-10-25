use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
#[derive(Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub hash: String,
    #[serde(default)]
    pub settings: DeviceSettings,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceSettings {
    pub cursor_speed: f64,
    pub scroll_speed: f64,
    pub show_points: bool,
    pub theme: String,
}
impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            cursor_speed: 1.0,
            scroll_speed: 1.0,
            show_points: false,
            theme: "system".into(),
        }
    }
}
impl DeviceSettings {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.cursor_speed.is_finite()
                && (0.25..=2.0).contains(&self.cursor_speed)
                && self.scroll_speed.is_finite()
                && (0.25..=2.0).contains(&self.scroll_speed)
                && ["system", "dark", "light"].contains(&self.theme.as_str()),
            "Invalid device settings"
        );
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub devices: Vec<Device>,
    pub background: bool,
    pub autostart: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            devices: vec![],
            background: true,
            autostart: false,
        }
    }
}
pub fn directory() -> PathBuf {
    std::env::var_os("GLIDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config"))
                .join("glide")
        })
}
pub fn load() -> Result<Config> {
    let p = directory().join("config.json");
    if !p.exists() {
        return Ok(Config::default());
    }
    let config: Config = serde_json::from_slice(&fs::read(p)?)?;
    for device in &config.devices {
        device.settings.validate()?;
    }
    Ok(config)
}
pub fn save(c: &Config) -> Result<()> {
    let dir = directory();
    fs::create_dir_all(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    let tmp = dir.join("config.tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(c)?)?;
    file.sync_all()?;
    fs::rename(tmp, dir.join("config.json"))?;
    Ok(())
}
pub fn digest(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}
pub fn token() -> String {
    use rand::RngCore;
    let mut b = [0u8; 32];
    rand::rng().fill_bytes(&mut b);
    hex::encode(b)
}
pub fn autostart(enable: bool) -> Result<()> {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config"));
    let path = root.join("autostart/io.github.glide.Touchpad.desktop");
    if !enable {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }
    let exe = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_exe()?);
    let exe = exe.to_str().context("Executable path is not UTF-8")?;
    anyhow::ensure!(!exe.contains(['\n', '\r']), "Unsupported executable path");
    let escaped = exe
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(
        path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Glide\nComment=Use your phone as a touchpad\nExec=\"{escaped}\" --background\nIcon=io.github.glide.Touchpad\nTerminal=false\n"
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_devices_get_default_settings() {
        let d: Device =
            serde_json::from_str(r#"{"id":"abc","name":"Phone","hash":"digest"}"#).unwrap();
        assert_eq!(d.settings, DeviceSettings::default());
    }
    #[test]
    fn reject_invalid_settings() {
        assert!(
            DeviceSettings {
                theme: "invalid".into(),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            DeviceSettings {
                scroll_speed: f64::NAN,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}
