use evdev::{InputEvent, KeyCode, RelativeAxisCode, uinput::VirtualDevice};
use std::time::Duration;

pub fn create_keyboard_device() -> Result<VirtualDevice, String> {
    let mut keys = evdev::AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_SPACE);
    keys.insert(KeyCode::KEY_W);
    keys.insert(KeyCode::KEY_A);
    keys.insert(KeyCode::KEY_S);
    keys.insert(KeyCode::KEY_D);
    keys.insert(KeyCode::KEY_I);
    keys.insert(KeyCode::KEY_O);

    VirtualDevice::builder()
        .map_err(|e: std::io::Error| e.to_string())?
        .name("AntiAFK Virtual Keyboard")
        .with_keys(&keys)
        .map_err(|e: std::io::Error| e.to_string())?
        .build()
        .map_err(|e: std::io::Error| {
            format!("Keyboard creation failed: {e}. Run: sudo chmod 666 /dev/uinput")
        })
}

pub fn create_mouse_device() -> Result<VirtualDevice, String> {
    let mut rel_axes = evdev::AttributeSet::<RelativeAxisCode>::new();
    rel_axes.insert(RelativeAxisCode::REL_X);
    rel_axes.insert(RelativeAxisCode::REL_Y);

    let mut keys = evdev::AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::BTN_LEFT);

    VirtualDevice::builder()
        .map_err(|e: std::io::Error| e.to_string())?
        .name("AntiAFK Virtual Mouse")
        .with_relative_axes(&rel_axes)
        .map_err(|e: std::io::Error| e.to_string())?
        .with_keys(&keys)
        .map_err(|e: std::io::Error| e.to_string())?
        .build()
        .map_err(|e: std::io::Error| format!("Mouse creation failed: {e}. Check permissions."))
}

pub fn emit_key(
    device: &mut VirtualDevice,
    key: KeyCode,
    pressed: bool,
) -> Result<(), std::io::Error> {
    device.emit(&[
        InputEvent::new(evdev::EventType::KEY.0, key.code(), i32::from(pressed)),
        InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
    ])
}

pub fn tap_key(
    device: &mut VirtualDevice,
    key: KeyCode,
    hold_duration: Duration,
) -> Result<(), std::io::Error> {
    emit_key(device, key, true)?;
    std::thread::sleep(hold_duration);
    emit_key(device, key, false)
}
