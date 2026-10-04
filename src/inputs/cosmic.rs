use crate::environment::InputMode;
use crate::input::create_keyboard_device;
use crate::inputs::common::{
    click_absolute_mouse, click_mouse, create_absolute_mouse_device, is_sober_metadata,
    move_absolute_mouse, perform_keyboard_actions, responsive_sleep, set_action_active, set_status,
    should_continue, wayland_capture_available,
};
use crate::state::{AppState, RuntimeStatus, SharedState};
use cosmic_client_toolkit::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1;
use cosmic_client_toolkit::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use cosmic_client_toolkit::sctk;
use cosmic_client_toolkit::sctk::output::{OutputHandler, OutputState};
use cosmic_client_toolkit::sctk::registry::{ProvidesRegistryState, RegistryState};
use cosmic_client_toolkit::toplevel_info::{ToplevelInfo, ToplevelInfoHandler, ToplevelInfoState};
use cosmic_client_toolkit::toplevel_management::{ToplevelManagerHandler, ToplevelManagerState};
use cosmic_client_toolkit::wayland_client::globals::registry_queue_init;
use cosmic_client_toolkit::wayland_client::protocol::{wl_output, wl_seat};
use cosmic_client_toolkit::wayland_client::{Connection, QueueHandle, WEnum};
use evdev::uinput::VirtualDevice;
use image::GenericImageView;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

static COSMIC_IS_SOBER: AtomicBool = AtomicBool::new(false);

struct CosmicData {
    registry: RegistryState,
    output_state: OutputState,
    toplevel_info: ToplevelInfoState,
    toplevel_manager: ToplevelManagerState,
    seat: wl_seat::WlSeat,
    capabilities:
        Vec<WEnum<zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1>>,
}

impl ProvidesRegistryState for CosmicData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    sctk::registry_handlers!(OutputState);
}

impl OutputHandler for CosmicData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ToplevelInfoHandler for CosmicData {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        &mut self.toplevel_info
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &cosmic_client_toolkit::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &cosmic_client_toolkit::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _toplevel: &cosmic_client_toolkit::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) {
    }

    fn info_done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

impl ToplevelManagerHandler for CosmicData {
    fn toplevel_manager_state(&mut self) -> &mut ToplevelManagerState {
        &mut self.toplevel_manager
    }

    fn capabilities(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        capabilities: Vec<
            WEnum<zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1>,
        >,
    ) {
        self.capabilities = capabilities;
    }
}

struct CosmicClient {
    data: CosmicData,
    queue: cosmic_client_toolkit::wayland_client::EventQueue<CosmicData>,
    _connection: Connection,
}

impl CosmicClient {
    fn open() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("COSMIC Wayland connection failed: {error}"))?;
        let (globals, mut queue) = registry_queue_init(&connection)
            .map_err(|error| format!("COSMIC Wayland registry failed: {error}"))?;
        let qh = queue.handle();
        let registry = RegistryState::new(&globals);
        let output_state = OutputState::new(&globals, &qh);
        let toplevel_info = ToplevelInfoState::try_new(&registry, &qh)
            .ok_or("COSMIC foreign-toplevel protocols are unavailable")?;
        let toplevel_manager = ToplevelManagerState::try_new(&registry, &qh)
            .ok_or("COSMIC toplevel-management protocol is unavailable")?;
        let seat = registry
            .bind_one::<wl_seat::WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|error| format!("COSMIC seat is unavailable: {error}"))?;
        let mut data = CosmicData {
            registry,
            output_state,
            toplevel_info,
            toplevel_manager,
            seat,
            capabilities: Vec::new(),
        };
        for _ in 0..20 {
            queue
                .roundtrip(&mut data)
                .map_err(|error| error.to_string())?;
            if !data.capabilities.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if data.capabilities.is_empty() {
            return Err("COSMIC toplevel manager did not advertise capabilities".to_string());
        }
        Ok(Self {
            data,
            queue,
            _connection: connection,
        })
    }

    fn roundtrip(&mut self) -> Result<(), String> {
        self.queue
            .roundtrip(&mut self.data)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn all_windows(&self) -> Vec<ToplevelInfo> {
        self.data.toplevel_info.toplevels().cloned().collect()
    }

    fn refresh(&mut self) -> Result<(), String> {
        self.roundtrip()
    }

    fn windows(&self) -> Vec<ToplevelInfo> {
        self.all_windows()
            .into_iter()
            .filter(|window| {
                is_sober_metadata(
                    Some(&window.app_id),
                    Some(&window.app_id),
                    Some(&window.title),
                )
            })
            .collect()
    }

    fn focused_toplevel(&self) -> Option<ToplevelInfo> {
        self.all_windows().into_iter().find(|window| {
            window
                .state
                .contains(&zcosmic_toplevel_handle_v1::State::Activated)
        })
    }

    fn focus_changed_during_probe(&mut self) -> Result<bool, String> {
        let before = self
            .focused_toplevel()
            .map(|window| window.foreign_toplevel);
        thread::sleep(Duration::from_secs(3));
        self.refresh()?;
        let after = self
            .focused_toplevel()
            .map(|window| window.foreign_toplevel);
        Ok(before != after)
    }

    fn supports_activate(&self) -> bool {
        self.data.capabilities.iter().any(|capability| {
            matches!(
                capability,
                WEnum::Value(
                    zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1::Activate
                )
            )
        })
    }

    fn focus(&mut self, window: &ToplevelInfo) -> Result<(), String> {
        if !self.supports_activate() {
            return Err("COSMIC compositor does not advertise toplevel activation".to_string());
        }
        let handle = window
            .cosmic_toplevel
            .as_ref()
            .ok_or("COSMIC toplevel handle is unavailable")?;
        self.data
            .toplevel_manager
            .manager
            .activate(handle, &self.data.seat);

        for _ in 0..5 {
            self.roundtrip()?;
            if self
                .data
                .toplevel_info
                .info(&window.foreign_toplevel)
                .is_some_and(|info| {
                    info.state
                        .contains(&zcosmic_toplevel_handle_v1::State::Activated)
                })
            {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    }

    fn screen_bounds(&self) -> (i32, i32, i32, i32) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for output in self.data.output_state.outputs() {
            let Some(info) = self.data.output_state.info(&output) else {
                continue;
            };
            let Some((x, y)) = info.logical_position else {
                continue;
            };
            let Some((width, height)) = info.logical_size else {
                continue;
            };
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + width);
            max_y = max_y.max(y + height);
        }
        if min_x == i32::MAX {
            (0, 0, 1920, 1080)
        } else {
            (min_x, min_y, max_x - min_x, max_y - min_y)
        }
    }

    fn window_rect(&self, window: &ToplevelInfo) -> Option<(i32, i32, i32, i32)> {
        let (output, geometry) = window
            .geometry
            .iter()
            .find(|(_, geometry)| geometry.width > 0 && geometry.height > 0)?;
        let info = self.data.output_state.info(output)?;
        let (output_x, output_y) = info.logical_position.unwrap_or((0, 0));
        Some((
            output_x + geometry.x,
            output_y + geometry.y,
            geometry.width,
            geometry.height,
        ))
    }

    fn click(&self, window: &ToplevelInfo, mouse: &mut VirtualDevice) -> Result<(), String> {
        let (screen_x, screen_y, screen_width, screen_height) = self.screen_bounds();
        for (output, geometry) in &window.geometry {
            if geometry.width <= 0 || geometry.height <= 0 {
                continue;
            }
            if let Some(info) = self.data.output_state.info(output) {
                let (output_x, output_y) = info.logical_position.unwrap_or((0, 0));
                let x = output_x + geometry.x + geometry.width / 2 - screen_x;
                let y = output_y + geometry.y + geometry.height / 2 - screen_y;
                return click_absolute_mouse(mouse, x, y, screen_width, screen_height);
            }
        }
        Err("COSMIC did not publish Sober window geometry".to_string())
    }

    fn set_hidden(&mut self, window: &ToplevelInfo, hidden: bool) -> Result<(), String> {
        let handle = window
            .cosmic_toplevel
            .as_ref()
            .ok_or("COSMIC toplevel handle is unavailable")?;
        let manager = &self.data.toplevel_manager.manager;
        if hidden {
            manager.set_minimized(handle);
        } else {
            manager.unset_minimized(handle);
        }
        self.roundtrip()
    }

    fn is_minimized(window: &ToplevelInfo) -> bool {
        window
            .state
            .contains(&zcosmic_toplevel_handle_v1::State::Minimized)
    }

    fn supports_hide(&self) -> bool {
        self.data.capabilities.iter().any(|capability| {
            matches!(
                capability,
                WEnum::Value(
                    zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1::Minimize
                )
            )
        })
    }
}

fn perform_cosmic_actions(settings: &AppState, keyboard: &mut VirtualDevice) -> Result<(), String> {
    perform_keyboard_actions(keyboard, settings)
}

fn auto_reconnect_with_mouse(
    state: &SharedState,
    mouse: &mut VirtualDevice,
    client: &CosmicClient,
    window: &ToplevelInfo,
) -> Result<bool, String> {
    set_status(state, RuntimeStatus::ReconnectChecking);
    let (window_x, window_y, window_width, window_height) = match client.window_rect(window) {
        Some(rect) => rect,
        None => return Ok(false),
    };
    let (_, _, screen_width, screen_height) = client.screen_bounds();

    let tmp_path = std::env::temp_dir().join("antiafk-cosmic-recon.png");
    let _ = std::fs::remove_file(&tmp_path);

    let geom_arg = format!("{window_x},{window_y} {window_width}x{window_height}");
    let capture_status = std::process::Command::new("grim")
        .args(["-g", &geom_arg, tmp_path.to_str().unwrap_or("")])
        .status();

    let img = match capture_status {
        Ok(status) if status.success() => image::open(&tmp_path).ok(),
        _ => None,
    };
    let _ = std::fs::remove_file(&tmp_path);

    let Some(img) = img else {
        set_status(state, RuntimeStatus::ReconnectNotFound);
        thread::sleep(Duration::from_millis(150));
        return Ok(false);
    };

    let (img_w, img_h) = (img.width(), img.height());
    if img_w < 50 || img_h < 50 {
        set_status(state, RuntimeStatus::ReconnectNotFound);
        thread::sleep(Duration::from_millis(150));
        return Ok(false);
    }

    let mut found = false;
    let cx = (img_w / 2) as i32;
    let cy = (img_h / 2) as i32;

    for dy in -40..=40 {
        for dx in -40..=40 {
            let px = (cx + dx).clamp(0, img_w as i32 - 1) as u32;
            let py = (cy + dy).clamp(0, img_h as i32 - 1) as u32;
            let p = img.get_pixel(px, py);
            if crate::inputs::common::reconnect_pixel_match(p[0], p[1], p[2]) {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }

    if !found {
        set_status(state, RuntimeStatus::ReconnectNotFound);
        thread::sleep(Duration::from_millis(150));
        return Ok(false);
    }

    set_status(state, RuntimeStatus::ReconnectFound);
    thread::sleep(Duration::from_millis(150));

    let (btn_x, btn_y) =
        crate::inputs::common::reconnect_button(window_x, window_y, window_width, window_height);

    let (screen_x, screen_y, _, _) = client.screen_bounds();
    let target_x = btn_x - screen_x;
    let target_y = btn_y - screen_y;

    move_absolute_mouse(mouse, target_x, target_y, screen_width, screen_height)?;
    thread::sleep(Duration::from_millis(100));
    set_status(state, RuntimeStatus::Reconnecting);
    for _ in 0..3 {
        click_mouse(mouse)?;
        thread::sleep(Duration::from_millis(30));
    }
    thread::sleep(Duration::from_millis(100));
    Ok(true)
}

pub fn probe() -> Result<(), String> {
    CosmicClient::open().map(|_| ())
}

pub fn supports_hide_mode() -> bool {
    CosmicClient::open()
        .map(|client| client.supports_hide())
        .unwrap_or(false)
}

pub fn supports_activate_mode() -> bool {
    CosmicClient::open()
        .map(|client| client.supports_activate())
        .unwrap_or(false)
}

pub fn supports_capture_mode() -> bool {
    wayland_capture_available()
}

pub fn show_sober() -> Result<(), String> {
    let mut client = CosmicClient::open()?;
    let target = client
        .windows()
        .into_iter()
        .next()
        .ok_or("Sober window was not found")?;
    client.focus(&target)
}

pub fn hide_sober() -> Result<(), String> {
    let mut client = CosmicClient::open()?;
    if !client.supports_hide() {
        return Err("COSMIC compositor does not advertise Hide Game support".to_string());
    }
    let target = client
        .windows()
        .into_iter()
        .next()
        .ok_or("Sober window was not found")?;
    client.set_hidden(&target, true)
}

pub fn focused_is_sober() -> bool {
    COSMIC_IS_SOBER.load(Ordering::Relaxed)
}

pub fn run(state: &SharedState) -> Result<(), String> {
    let mut client = CosmicClient::open()?;
    let mut keyboard = create_keyboard_device()?;
    let mut mouse = create_absolute_mouse_device()?;
    if !client.supports_hide() && { state.lock().unwrap().stealth } {
        return Err("COSMIC compositor does not advertise Hide Game support".to_string());
    }
    let mut hidden_targets: Vec<ToplevelInfo> = Vec::new();

    loop {
        if !should_continue(state, InputMode::Cosmic) {
            break;
        }
        let settings = { state.lock().unwrap().clone() };
        if !settings.stealth {
            for target in hidden_targets.drain(..) {
                let _ = client.set_hidden(&target, false);
            }
        }
        if client.refresh().is_err()
            && let Ok(new_client) = CosmicClient::open()
        {
            client = new_client;
        }
        let is_sober = client.all_windows().into_iter().any(|window| {
            window
                .state
                .contains(&zcosmic_toplevel_handle_v1::State::Activated)
                && is_sober_metadata(
                    Some(&window.app_id),
                    Some(&window.app_id),
                    Some(&window.title),
                )
        });
        COSMIC_IS_SOBER.store(is_sober, Ordering::Relaxed);

        if settings.user_safe {
            if crate::inputs::common::has_recent_input(3) {
                set_status(state, RuntimeStatus::Paused);
                if responsive_sleep(state, InputMode::Cosmic, 3) {
                    break;
                }
                continue;
            }
            match client.focus_changed_during_probe() {
                Ok(true) => {
                    set_status(state, RuntimeStatus::Paused);
                    if responsive_sleep(state, InputMode::Cosmic, 5) {
                        break;
                    }
                    continue;
                }
                Ok(false) => {}
                Err(error) => return Err(error),
            }
        }
        let mut targets = client.windows();
        if !settings.multi_instance {
            targets.truncate(1);
        }
        if targets.is_empty() {
            set_status(state, RuntimeStatus::WaitingForSober);
            if responsive_sleep(state, InputMode::Cosmic, 2) {
                break;
            }
            continue;
        }

        let previous = client.all_windows().into_iter().find(|window| {
            window
                .state
                .contains(&zcosmic_toplevel_handle_v1::State::Activated)
        });
        set_action_active(state, true);
        set_status(state, RuntimeStatus::PerformingAction);
        thread::sleep(Duration::from_millis(500));
        let mut error = None;
        for target in targets {
            if !should_continue(state, InputMode::Cosmic) {
                break;
            }
            let was_minimized = CosmicClient::is_minimized(&target);
            if was_minimized && let Err(err) = client.set_hidden(&target, false) {
                error = Some(err);
                break;
            }
            if let Err(err) = client.focus(&target) {
                error = Some(err);
                break;
            }
            if let Err(err) = client.click(&target, &mut mouse) {
                error = Some(err);
                break;
            }
            if let Err(err) = perform_cosmic_actions(&settings, &mut keyboard) {
                error = Some(err);
                break;
            }
            if settings.auto_reconnect
                && let Err(err) = auto_reconnect_with_mouse(state, &mut mouse, &client, &target)
            {
                error = Some(err);
                break;
            }
            if (settings.stealth || was_minimized)
                && let Err(err) = client.set_hidden(&target, true)
            {
                error = Some(err);
                break;
            }
            if settings.stealth
                && !hidden_targets
                    .iter()
                    .any(|hidden| hidden.foreign_toplevel == target.foreign_toplevel)
            {
                hidden_targets.push(target);
            }
        }
        let orig_pos = previous.as_ref().and_then(|p| client.window_rect(p));
        if let Some(previous) = previous
            && !hidden_targets
                .iter()
                .any(|hidden| hidden.foreign_toplevel == previous.foreign_toplevel)
        {
            let _ = client.focus(&previous);
        }
        if let Some((ox, oy, ow, oh)) = orig_pos {
            let (screen_x, screen_y, screen_width, screen_height) = client.screen_bounds();
            let prev_center_x = ox + ow / 2 - screen_x;
            let prev_center_y = oy + oh / 2 - screen_y;
            let _ = move_absolute_mouse(
                &mut mouse,
                prev_center_x,
                prev_center_y,
                screen_width,
                screen_height,
            );
        }
        set_action_active(state, false);
        if let Some(error) = error {
            for target in hidden_targets.drain(..) {
                let _ = client.set_hidden(&target, false);
            }
            return Err(error);
        }
        set_status(state, RuntimeStatus::Ready);
        if responsive_sleep(state, InputMode::Cosmic, settings.interval_seq) {
            break;
        }
    }

    COSMIC_IS_SOBER.store(false, Ordering::Relaxed);
    for target in hidden_targets.drain(..) {
        let _ = client.set_hidden(&target, false);
    }
    set_action_active(state, false);
    set_status(state, RuntimeStatus::Stopped);
    Ok(())
}

#[allow(dead_code)]
fn _state_type(_: zcosmic_toplevel_handle_v1::State) {}

sctk::delegate_registry!(CosmicData);
sctk::delegate_output!(CosmicData);
cosmic_client_toolkit::delegate_toplevel_info!(CosmicData);
cosmic_client_toolkit::delegate_toplevel_manager!(CosmicData);
cosmic_client_toolkit::wayland_client::delegate_noop!(CosmicData: ignore wl_seat::WlSeat);
