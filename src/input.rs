use anyhow::{Result, bail};
use evdev::{
    AbsInfo, AbsoluteAxisCode as A, AttributeSet, BusType, EventType, InputEvent, InputId,
    KeyCode as K, PropType, UinputAbsSetup, uinput::VirtualDevice,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{self, BufRead},
    sync::mpsc,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point {
    pub id: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Surface {
    pub width: f64,
    pub height: f64,
}
impl Default for Surface {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 750.0,
        }
    }
}
impl Surface {
    fn map(&self, p: &Point) -> Point {
        // Fit the browser surface inside the virtual pad with one units/pixel
        // scale. Centering leaves equal headroom for amplified motion.
        let scale = (12000.0 / self.width).min(7500.0 / self.height);
        Point {
            id: p.id,
            x: 0.5 + (p.x - 0.5) * self.width * scale / 12000.0,
            y: 0.5 + (p.y - 0.5) * self.height * scale / 7500.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    PlayPause,
    Previous,
    Next,
    VolumeUp,
    VolumeDown,
}
impl MediaAction {
    fn key(self) -> K {
        match self {
            Self::PlayPause => K::KEY_PLAYPAUSE,
            Self::Previous => K::KEY_PREVIOUSSONG,
            Self::Next => K::KEY_NEXTSONG,
            Self::VolumeUp => K::KEY_VOLUMEUP,
            Self::VolumeDown => K::KEY_VOLUMEDOWN,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeypadKey {
    #[serde(rename = "0")]
    Digit0,
    #[serde(rename = "1")]
    Digit1,
    #[serde(rename = "2")]
    Digit2,
    #[serde(rename = "3")]
    Digit3,
    #[serde(rename = "4")]
    Digit4,
    #[serde(rename = "5")]
    Digit5,
    #[serde(rename = "6")]
    Digit6,
    #[serde(rename = "7")]
    Digit7,
    #[serde(rename = "8")]
    Digit8,
    #[serde(rename = "9")]
    Digit9,
    Decimal,
    Add,
    Subtract,
    Multiply,
    Divide,
    Enter,
    Backspace,
    Delete,
    Tab,
    NumLock,
}
const KEYPAD_KEYS: [KeypadKey; 20] = [
    KeypadKey::Digit0,
    KeypadKey::Digit1,
    KeypadKey::Digit2,
    KeypadKey::Digit3,
    KeypadKey::Digit4,
    KeypadKey::Digit5,
    KeypadKey::Digit6,
    KeypadKey::Digit7,
    KeypadKey::Digit8,
    KeypadKey::Digit9,
    KeypadKey::Decimal,
    KeypadKey::Add,
    KeypadKey::Subtract,
    KeypadKey::Multiply,
    KeypadKey::Divide,
    KeypadKey::Enter,
    KeypadKey::Backspace,
    KeypadKey::Delete,
    KeypadKey::Tab,
    KeypadKey::NumLock,
];
impl KeypadKey {
    fn key(self) -> K {
        match self {
            Self::Digit0 => K::KEY_KP0,
            Self::Digit1 => K::KEY_KP1,
            Self::Digit2 => K::KEY_KP2,
            Self::Digit3 => K::KEY_KP3,
            Self::Digit4 => K::KEY_KP4,
            Self::Digit5 => K::KEY_KP5,
            Self::Digit6 => K::KEY_KP6,
            Self::Digit7 => K::KEY_KP7,
            Self::Digit8 => K::KEY_KP8,
            Self::Digit9 => K::KEY_KP9,
            Self::Decimal => K::KEY_KPDOT,
            Self::Add => K::KEY_KPPLUS,
            Self::Subtract => K::KEY_KPMINUS,
            Self::Multiply => K::KEY_KPASTERISK,
            Self::Divide => K::KEY_KPSLASH,
            Self::Enter => K::KEY_KPENTER,
            Self::Backspace => K::KEY_BACKSPACE,
            Self::Delete => K::KEY_DELETE,
            Self::Tab => K::KEY_TAB,
            Self::NumLock => K::KEY_NUMLOCK,
        }
    }
}
fn media_device() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::new();
    for action in [
        MediaAction::PlayPause,
        MediaAction::Previous,
        MediaAction::Next,
        MediaAction::VolumeUp,
        MediaAction::VolumeDown,
    ] {
        keys.insert(action.key());
    }
    for key in KEYPAD_KEYS {
        keys.insert(key.key());
    }
    Ok(VirtualDevice::builder()?
        .name("Glide Controls")
        .input_id(InputId::new(BusType::BUS_USB, 0x1209, 0x474d, 1))
        .with_keys(&keys)?
        .build()?)
}
fn emit_media(device: &mut VirtualDevice, action: MediaAction) -> Result<()> {
    // Separate reports ensure both transitions are visible; never leave a key held.
    device.emit(&[InputEvent::new(EventType::KEY.0, action.key().0, 1)])?;
    device.emit(&[InputEvent::new(EventType::KEY.0, action.key().0, 0)])?;
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    #[serde(default)]
    pub points: Vec<Point>,
    #[serde(default)]
    pub button: u8,
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub media: Option<MediaAction>,
    #[serde(default)]
    pub keypad: Option<KeypadKey>,
    #[serde(default)]
    pub surface: Surface,
    #[serde(default = "normal_speed")]
    pub cursor_speed: f64,
    #[serde(default = "normal_speed")]
    pub scroll_speed: f64,
}
fn normal_speed() -> f64 {
    1.0
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            button: 0,
            seq: 0,
            media: None,
            keypad: None,
            surface: Surface::default(),
            cursor_speed: 1.0,
            scroll_speed: 1.0,
        }
    }
}
impl Frame {
    pub fn validate(&self) -> Result<()> {
        if ((self.media.is_some() || self.keypad.is_some())
            && (!self.points.is_empty() || self.button != 0))
            || (self.media.is_some() && self.keypad.is_some())
            || !self.surface.width.is_finite()
            || !self.surface.height.is_finite()
            || !(1.0..=16384.0).contains(&self.surface.width)
            || !(1.0..=16384.0).contains(&self.surface.height)
            || self.points.len() > 5
            || self.button > 1
            || !self.cursor_speed.is_finite()
            || !(0.25..=2.0).contains(&self.cursor_speed)
            || !self.scroll_speed.is_finite()
            || !(0.25..=2.0).contains(&self.scroll_speed)
        {
            bail!("Invalid frame");
        }
        let mut ids = HashSet::new();
        for p in &self.points {
            if !ids.insert(p.id)
                || !p.x.is_finite()
                || !p.y.is_finite()
                || !(0.0..=1.0).contains(&p.x)
                || !(0.0..=1.0).contains(&p.y)
            {
                bail!("Invalid contact");
            }
        }
        Ok(())
    }
}
const TOOLS: [K; 5] = [
    K::BTN_TOOL_FINGER,
    K::BTN_TOOL_DOUBLETAP,
    K::BTN_TOOL_TRIPLETAP,
    K::BTN_TOOL_QUADTAP,
    K::BTN_TOOL_QUINTTAP,
];
// Integrate movement, rather than multiplying absolute coordinates. Retain each
// contact's position when fingers are added/removed so gesture changes do not jump.
#[derive(Default)]
struct Motion {
    previous: BTreeMap<u32, Point>,
    output: BTreeMap<u32, Point>,
}
impl Motion {
    fn apply(&mut self, frame: &Frame) -> Vec<Point> {
        let same = frame.points.len() == self.previous.len()
            && frame
                .points
                .iter()
                .all(|p| self.previous.contains_key(&p.id));
        let mut dx = 0.0;
        let mut dy = 0.0;
        if same && !frame.points.is_empty() {
            for p in &frame.points {
                let old = &self.previous[&p.id];
                dx += p.x - old.x;
                dy += p.y - old.y;
            }
            dx /= frame.points.len() as f64;
            dy /= frame.points.len() as f64;
        }
        let gain = match frame.points.len() {
            1 => frame.cursor_speed,
            2 => frame.scroll_speed,
            _ => 1.0,
        };
        let result: Vec<Point> = frame
            .points
            .iter()
            .map(|p| {
                let mut next = p.clone();
                if let (Some(old), Some(out)) = (self.previous.get(&p.id), self.output.get(&p.id)) {
                    // Scale only common translation: separation/rotation for pinch stays native.
                    next.x = out.x + p.x - old.x + (gain - 1.0) * dx;
                    next.y = out.y + p.y - old.y + (gain - 1.0) * dy;
                }
                next
            })
            .collect();
        self.previous = frame.points.iter().map(|p| (p.id, p.clone())).collect();
        self.output = result.iter().map(|p| (p.id, p.clone())).collect();
        result
    }
}
#[derive(Default)]
pub struct Encoder {
    slots: BTreeMap<u32, i32>,
    tracking: i32,
    motion: Motion,
}
impl Encoder {
    pub fn encode(&mut self, f: &Frame) -> Result<Vec<InputEvent>> {
        f.validate()?;
        let mapped = Frame {
            points: f.points.iter().map(|p| f.surface.map(p)).collect(),
            ..f.clone()
        };
        let adjusted = Frame {
            points: self.motion.apply(&mapped),
            ..f.clone()
        };
        let f = &adjusted;
        let mut out = vec![InputEvent::new(
            EventType::KEY.0,
            K::BTN_TOUCH.0,
            (!f.points.is_empty()) as i32,
        )];
        let mut abs =
            |code: A, value: i32| out.push(InputEvent::new(EventType::ABSOLUTE.0, code.0, value));
        let removed: Vec<_> = self
            .slots
            .keys()
            .filter(|id| !f.points.iter().any(|p| p.id == **id))
            .copied()
            .collect();
        for id in removed {
            abs(A::ABS_MT_SLOT, self.slots.remove(&id).unwrap());
            abs(A::ABS_MT_TRACKING_ID, -1);
        }
        for p in &f.points {
            let new = !self.slots.contains_key(&p.id);
            if new {
                let slot = (0..5)
                    .find(|i| !self.slots.values().any(|v| v == i))
                    .unwrap();
                self.slots.insert(p.id, slot);
            }
            abs(A::ABS_MT_SLOT, self.slots[&p.id]);
            if new {
                self.tracking = (self.tracking + 1) % 65536;
                abs(A::ABS_MT_TRACKING_ID, self.tracking);
            }
            abs(A::ABS_MT_POSITION_X, (p.x * 12000.0).round() as i32);
            abs(A::ABS_MT_POSITION_Y, (p.y * 7500.0).round() as i32);
        }
        if let Some(p) = f.points.first() {
            abs(A::ABS_X, (p.x * 12000.0).round() as i32);
            abs(A::ABS_Y, (p.y * 7500.0).round() as i32);
        }
        for (i, k) in TOOLS.iter().enumerate() {
            out.push(InputEvent::new(
                EventType::KEY.0,
                k.0,
                (f.points.len() == i + 1) as i32,
            ));
        }
        out.push(InputEvent::new(
            EventType::KEY.0,
            K::BTN_LEFT.0,
            f.button as i32,
        ));
        Ok(out)
    }
}
fn device() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::new();
    for k in [K::BTN_TOUCH, K::BTN_LEFT].into_iter().chain(TOOLS) {
        keys.insert(k);
    }
    let mut props = AttributeSet::new();
    props.insert(PropType::POINTER);
    props.insert(PropType::BUTTONPAD);
    let mut b = VirtualDevice::builder()?
        .name("Glide Touchpad")
        .input_id(InputId::new(BusType::BUS_USB, 0x1209, 0x474c, 1))
        .with_keys(&keys)?
        .with_properties(&props)?;
    // Keep the same units/mm (and therefore the 1x feel), but reserve space
    // around the browser's [0,1] area for amplified contact movement.
    for (a, min, max, res) in [
        (A::ABS_X, -24000, 36000, 100),
        (A::ABS_Y, -15000, 22500, 100),
        (A::ABS_MT_SLOT, 0, 4, 0),
        (A::ABS_MT_TRACKING_ID, 0, 65535, 0),
        (A::ABS_MT_POSITION_X, -24000, 36000, 100),
        (A::ABS_MT_POSITION_Y, -15000, 22500, 100),
    ] {
        b = b.with_absolute_axis(&UinputAbsSetup::new(
            a,
            AbsInfo::new(0, min, max, 0, 0, res),
        ))?;
    }
    Ok(b.build()?)
}
fn num_lock_state(directory: &std::path::Path) -> Option<bool> {
    // Read status only, never physical keyboard input. Desktops normally keep
    // Num Lock LEDs synchronized. Conflicting/missing state is not a safe toggle.
    let mut state = None;
    for entry in std::fs::read_dir(directory).ok()? {
        let entry = entry.ok()?;
        if !entry.file_name().to_string_lossy().ends_with("::numlock") {
            continue;
        }
        let value = match std::fs::read_to_string(entry.path().join("brightness"))
            .ok()?
            .trim()
        {
            "0" => false,
            "1" => true,
            _ => return None,
        };
        if state.is_some_and(|old| old != value) {
            return None;
        }
        state = Some(value);
    }
    state
}
fn ensure_num_lock(
    read: impl Fn() -> Option<bool>,
    mut press: impl FnMut() -> Result<()>,
) -> Result<Option<bool>> {
    match read() {
        Some(false) => {}
        state => return Ok(state),
    }
    press()?;
    for _ in 0..15 {
        std::thread::sleep(Duration::from_millis(10));
        if read() == Some(true) {
            return Ok(Some(true));
        }
    }
    Ok(read())
}
pub fn helper() -> Result<()> {
    let mut device = device()?;
    let mut media = media_device()?;
    eprintln!("MEDIA {}", media.get_syspath()?.display());
    let mut encoder = Encoder::default();
    println!("READY {}", device.get_syspath()?.display());
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let mut reader = io::stdin().lock();
        loop {
            let mut line = String::new();
            // Bound each record before JSON parsing.
            let result = std::io::Read::take(&mut reader, 8193).read_line(&mut line);
            if !matches!(result, Ok(1..)) || line.len() > 8192 || !line.ends_with('\n') {
                break;
            }
            let Ok(frame) = serde_json::from_str::<Frame>(&line) else {
                break;
            };
            if frame.validate().is_err() || tx.send(frame).is_err() {
                break;
            }
        }
    });
    loop {
        match rx.recv_timeout(Duration::from_millis(1500)) {
            Ok(f) => {
                if let Some(action) = f.media {
                    emit_media(&mut media, action)?;
                } else if let Some(key) = f.keypad {
                    // Key actions are independent of a live touchpad stroke.
                    if key == KeypadKey::NumLock {
                        let state = ensure_num_lock(
                            || num_lock_state(std::path::Path::new("/sys/class/leds")),
                            || {
                                media.emit(&[InputEvent::new(
                                    EventType::KEY.0,
                                    K::KEY_NUMLOCK.0,
                                    1,
                                )])?;
                                media.emit(&[InputEvent::new(
                                    EventType::KEY.0,
                                    K::KEY_NUMLOCK.0,
                                    0,
                                )])?;
                                Ok(())
                            },
                        )?;
                        println!(
                            "{}",
                            serde_json::json!({"num_lock_state":match state {Some(true)=>"on",Some(false)=>"off",None=>"unknown"},"seq":f.seq})
                        );
                    } else {
                        media.emit(&[InputEvent::new(EventType::KEY.0, key.key().0, 1)])?;
                        media.emit(&[InputEvent::new(EventType::KEY.0, key.key().0, 0)])?;
                    }
                } else {
                    device.emit(&encoder.encode(&f)?)?;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                device.emit(&encoder.encode(&Frame::default())?)?
            }
            Err(_) => break,
        }
    }
    device.emit(&encoder.encode(&Frame::default())?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> Frame {
        Frame {
            points: (0..3)
                .map(|id| Point {
                    id,
                    x: 0.2 + id as f64 * 0.1,
                    y: 0.4,
                })
                .collect(),
            ..Default::default()
        }
    }
    #[test]
    fn enabling_num_lock_is_conditional_and_never_guesses() {
        use std::cell::Cell;
        let state = Cell::new(Some(true));
        let presses = Cell::new(0);
        let result = ensure_num_lock(
            || state.get(),
            || {
                presses.set(presses.get() + 1);
                state.set(Some(true));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, Some(true));
        assert_eq!(presses.get(), 0);
        state.set(Some(false));
        assert_eq!(
            ensure_num_lock(
                || state.get(),
                || {
                    presses.set(presses.get() + 1);
                    state.set(Some(true));
                    Ok(())
                }
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(presses.get(), 1);
        assert_eq!(
            ensure_num_lock(
                || state.get(),
                || {
                    presses.set(presses.get() + 1);
                    Ok(())
                }
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(presses.get(), 1);
        state.set(None);
        assert_eq!(
            ensure_num_lock(
                || state.get(),
                || {
                    presses.set(presses.get() + 1);
                    Ok(())
                }
            )
            .unwrap(),
            None
        );
        assert_eq!(presses.get(), 1);
        let reads = Cell::new(0);
        let transient = ensure_num_lock(
            || {
                let n = reads.get();
                reads.set(n + 1);
                match n {
                    0 => Some(false),
                    1 => None,
                    _ => Some(true),
                }
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(transient, Some(true));
    }
    #[test]
    fn num_lock_leds_require_consistent_reported_state() {
        let root = std::env::temp_dir().join(format!("glide-led-test-{}", crate::config::token()));
        std::fs::create_dir(&root).unwrap();
        assert_eq!(num_lock_state(&root), None);
        for id in ["input1::numlock", "input2::numlock"] {
            std::fs::create_dir(root.join(id)).unwrap();
            std::fs::write(root.join(id).join("brightness"), "1\n").unwrap();
        }
        assert_eq!(num_lock_state(&root), Some(true));
        std::fs::write(root.join("input1::numlock/brightness"), "0\n").unwrap();
        assert_eq!(num_lock_state(&root), None);
        std::fs::write(root.join("input2::numlock/brightness"), "0\n").unwrap();
        assert_eq!(num_lock_state(&root), Some(false));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn keypad_protocol_is_allowlisted_and_separate_from_touch_and_media() {
        for key in KEYPAD_KEYS {
            let name = serde_json::to_value(key).unwrap();
            let f: Frame = serde_json::from_value(serde_json::json!({"keypad":name})).unwrap();
            assert!(f.validate().is_ok());
            assert_eq!(f.keypad.unwrap().key(), key.key());
            let mut mixed = frame();
            mixed.keypad = f.keypad;
            assert!(mixed.validate().is_err());
            mixed.points.clear();
            mixed.media = Some(MediaAction::PlayPause);
            assert!(mixed.validate().is_err());
        }
        assert!(serde_json::from_str::<Frame>(r#"{"keypad":"shell"}"#).is_err());
        assert!(
            serde_json::from_str::<Frame>(r#"{"points":[]}"#)
                .unwrap()
                .keypad
                .is_none()
        );
    }
    #[test]
    fn media_protocol_is_bounded_and_separate_from_contacts() {
        for (name, key) in [
            ("play_pause", K::KEY_PLAYPAUSE),
            ("previous", K::KEY_PREVIOUSSONG),
            ("next", K::KEY_NEXTSONG),
            ("volume_up", K::KEY_VOLUMEUP),
            ("volume_down", K::KEY_VOLUMEDOWN),
        ] {
            let f: Frame = serde_json::from_value(serde_json::json!({"media":name})).unwrap();
            assert!(f.validate().is_ok());
            assert_eq!(f.media.unwrap().key(), key);
            let mut mixed = frame();
            mixed.media = f.media;
            assert!(mixed.validate().is_err());
        }
        assert!(serde_json::from_str::<Frame>(r#"{"media":"shell"}"#).is_err());
        assert!(
            serde_json::from_str::<Frame>(r#"{"points":[]}"#)
                .unwrap()
                .media
                .is_none()
        );
    }
    #[test]
    fn motion_gains_preserve_gestures_and_release() {
        let mut m = Motion::default();
        let mut f = Frame {
            points: vec![Point {
                id: 1,
                x: 0.4,
                y: 0.4,
            }],
            cursor_speed: 2.0,
            scroll_speed: 0.5,
            ..Default::default()
        };
        m.apply(&f);
        f.points[0].x += 0.1;
        assert!((m.apply(&f)[0].x - 0.6).abs() < 1e-9);
        // Heartbeats must never repeat the movement.
        assert!((m.apply(&f)[0].x - 0.6).abs() < 1e-9);
        // Adding a finger keeps existing contacts still.
        f.points.push(Point {
            id: 2,
            x: 0.7,
            y: 0.4,
        });
        assert!((m.apply(&f)[0].x - 0.6).abs() < 1e-9);
        for p in &mut f.points {
            p.y += 0.1;
        }
        let result = m.apply(&f);
        assert!(result.iter().all(|p| (p.y - 0.45).abs() < 1e-9));
        // Opposing pinch movement has zero centroid delta: do not scale it.
        f.points[0].x -= 0.03;
        f.points[1].x += 0.03;
        let pinch = m.apply(&f);
        assert!(((pinch[1].x - pinch[0].x) - (result[1].x - result[0].x) - 0.06).abs() < 1e-9);
        f.points.push(Point {
            id: 3,
            x: 0.8,
            y: 0.5,
        });
        m.apply(&f);
        let before = m.apply(&f);
        for p in &mut f.points {
            p.y += 0.02;
        }
        let after = m.apply(&f);
        assert!(
            after
                .iter()
                .zip(&before)
                .all(|(a, b)| (a.y - b.y - 0.02).abs() < 1e-9)
        );
        m.apply(&Frame::default());
        assert!(m.previous.is_empty() && m.output.is_empty());
        assert_eq!(m.apply(&f)[0].x, f.points[0].x);
    }
    #[test]
    fn speed_validation_and_edge_reversal() {
        let old: Frame = serde_json::from_str(r#"{"points":[]}"#).unwrap();
        assert_eq!(old.cursor_speed, 1.0);
        assert_eq!(old.scroll_speed, 1.0);
        for invalid in [0.0, 2.1, f64::INFINITY, f64::NAN] {
            assert!(
                Frame {
                    cursor_speed: invalid,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
            assert!(
                Frame {
                    scroll_speed: invalid,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        let mut m = Motion::default();
        let mut f = Frame {
            points: vec![Point {
                id: 0,
                x: 0.9,
                y: 0.5,
            }],
            cursor_speed: 2.0,
            ..Default::default()
        };
        m.apply(&f);
        f.points[0].x = 1.0;
        assert!((m.apply(&f)[0].x - 1.1).abs() < 1e-9);
        f.points[0].x = 0.95;
        assert!((m.apply(&f)[0].x - 1.0).abs() < 1e-9);
    }
    #[test]
    fn full_width_gain_never_stops_at_the_old_edge() {
        for fingers in [1, 2] {
            let mut e = Encoder::default();
            let mut previous = None;
            for step in 0..=100 {
                let f = Frame {
                    points: (0..fingers)
                        .map(|id| Point {
                            id,
                            x: step as f64 / 100.0,
                            y: 0.4 + id as f64 * 0.1,
                        })
                        .collect(),
                    cursor_speed: 2.0,
                    scroll_speed: 2.0,
                    ..Default::default()
                };
                let out = e.encode(&f).unwrap();
                let x = out
                    .iter()
                    .find(|v| v.code() == A::ABS_MT_POSITION_X.0)
                    .unwrap()
                    .value();
                if let Some(last) = previous {
                    assert_eq!(x - last, 240, "Movement stopped at {step}%");
                }
                assert!((-24000..=36000).contains(&x));
                previous = Some(x);
            }
            assert_eq!(previous, Some(24000));
        }
    }
    #[test]
    fn equal_pixel_travel_has_equal_axis_motion() {
        for (width, height) in [(390.0, 844.0), (844.0, 390.0), (600.0, 600.0)] {
            for gain in [0.25, 1.0, 2.0] {
                let travel = |vertical: bool| {
                    let mut encoder = Encoder::default();
                    let mut frame = Frame {
                        surface: Surface { width, height },
                        cursor_speed: gain,
                        points: vec![Point {
                            id: 0,
                            x: 0.4,
                            y: 0.4,
                        }],
                        ..Default::default()
                    };
                    let code = if vertical {
                        A::ABS_MT_POSITION_Y
                    } else {
                        A::ABS_MT_POSITION_X
                    };
                    let position = |events: Vec<InputEvent>| {
                        events
                            .into_iter()
                            .find(|e| e.code() == code.0)
                            .unwrap()
                            .value()
                    };
                    let start = position(encoder.encode(&frame).unwrap());
                    if vertical {
                        frame.points[0].y += 60.0 / height;
                    } else {
                        frame.points[0].x += 60.0 / width;
                    }
                    position(encoder.encode(&frame).unwrap()) - start
                };
                assert!(
                    (travel(false) - travel(true)).abs() <= 1,
                    "{width}x{height} at {gain}x"
                );
            }
        }
    }
    #[test]
    fn invalid_surface_is_rejected() {
        for value in [0.0, -1.0, 16385.0, f64::NAN, f64::INFINITY] {
            let mut f = Frame::default();
            f.surface.width = value;
            assert!(f.validate().is_err());
            f = Frame::default();
            f.surface.height = value;
            assert!(f.validate().is_err());
        }
    }
    #[test]
    fn stable_slots_and_release() {
        let mut e = Encoder::default();
        let mut f = frame();
        e.encode(&f).unwrap();
        let slots = e.slots.clone();
        f.points.reverse();
        let out = e.encode(&f).unwrap();
        assert_eq!(e.slots, slots);
        assert!(!out.iter().any(|e| e.code() == A::ABS_MT_TRACKING_ID.0));
        let out = e.encode(&Frame::default()).unwrap();
        assert_eq!(
            out.iter()
                .filter(|e| e.code() == A::ABS_MT_TRACKING_ID.0 && e.value() == -1)
                .count(),
            3
        );
        assert!(e.slots.is_empty());
    }
    #[test]
    fn invalid_is_atomic() {
        let mut e = Encoder::default();
        let mut f = frame();
        f.points[0].x = f64::NAN;
        assert!(e.encode(&f).is_err());
        assert!(e.slots.is_empty());
        f.points[0].x = 0.5;
        f.points[1].id = f.points[0].id;
        assert!(e.encode(&f).is_err());
    }
}
