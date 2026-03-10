use evdev::{
    InputEvent, Key, RelativeAxisType, uinput::VirtualDevice, uinput::VirtualDeviceBuilder,
};

pub fn create_keyboard_device() -> Result<VirtualDevice, String> {
    let mut keys = evdev::AttributeSet::<Key>::new();
    keys.insert(Key::KEY_SPACE);
    keys.insert(Key::KEY_W);
    keys.insert(Key::KEY_A);
    keys.insert(Key::KEY_S);
    keys.insert(Key::KEY_D);

    VirtualDeviceBuilder::new()
        .map_err(|e: std::io::Error| e.to_string())?
        .name("AntiAFK Virtual Keyboard")
        .with_keys(&keys)
        .map_err(|e: std::io::Error| e.to_string())?
        .build()
        .map_err(|e: std::io::Error| format!("Creation failed: {e}. Check permissions."))
}

pub fn create_mouse_device() -> Result<VirtualDevice, String> {
    let mut rel_axes = evdev::AttributeSet::<RelativeAxisType>::new();
    rel_axes.insert(RelativeAxisType::REL_X);
    rel_axes.insert(RelativeAxisType::REL_Y);

    let mut keys = evdev::AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);

    VirtualDeviceBuilder::new()
        .map_err(|e: std::io::Error| e.to_string())?
        .name("AntiAFK Virtual Mouse")
        .with_relative_axes(&rel_axes)
        .map_err(|e: std::io::Error| e.to_string())?
        .with_keys(&keys)
        .map_err(|e: std::io::Error| e.to_string())?
        .build()
        .map_err(|e: std::io::Error| format!("Creation failed: {e}. Check permissions."))
}

pub fn emit_key(device: &mut VirtualDevice, key: Key, pressed: bool) -> Result<(), std::io::Error> {
    device.emit(&[
        InputEvent::new(evdev::EventType::KEY, key.code(), i32::from(pressed)),
        InputEvent::new(evdev::EventType::SYNCHRONIZATION, 0, 0),
    ])
}
