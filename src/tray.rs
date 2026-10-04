use crate::state::{APP_ID, APP_TITLE, SharedState};
use image::GenericImageView;
use ksni::blocking::{Handle, TrayMethods};
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Show,
    ShowSober,
    HideSober,
    Quit,
}

fn load_icon(data: &[u8]) -> Vec<ksni::Icon> {
    if let Ok(img) = image::load_from_memory(data) {
        let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8();
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for pixel in rgba.pixels() {
            pixels.push(pixel[3]);
            pixels.push(pixel[0]);
            pixels.push(pixel[1]);
            pixels.push(pixel[2]);
        }
        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data: pixels,
        }]
    } else {
        vec![]
    }
}

static TRAY_OFF_ICON: LazyLock<Vec<ksni::Icon>> =
    LazyLock::new(|| load_icon(include_bytes!("../assets/tray-icons/tray-off.png")));
static TRAY_RUN_ICON: LazyLock<Vec<ksni::Icon>> =
    LazyLock::new(|| load_icon(include_bytes!("../assets/tray-icons/tray-on.png")));

struct AntiAFKTray {
    queue: Arc<Mutex<Vec<TrayCommand>>>,
    state: SharedState,
}

impl AntiAFKTray {
    fn item(&self, label: &str, command: TrayCommand) -> ksni::MenuItem<Self> {
        use ksni::menu::StandardItem;
        let queue = self.queue.clone();
        ksni::MenuItem::Standard(StandardItem {
            label: label.into(),
            activate: Box::new(move |_| {
                queue.lock().unwrap().push(command);
            }),
            ..Default::default()
        })
    }
}

impl ksni::Tray for AntiAFKTray {
    fn icon_name(&self) -> String {
        String::new()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let is_running = self
            .state
            .lock()
            .map(|state| state.running)
            .unwrap_or(false);
        if is_running {
            TRAY_RUN_ICON.clone()
        } else {
            TRAY_OFF_ICON.clone()
        }
    }

    fn id(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        APP_TITLE.into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            self.item("Show Window", TrayCommand::Show),
            ksni::MenuItem::Separator,
            self.item("Show Sober", TrayCommand::ShowSober),
            self.item("Hide Sober", TrayCommand::HideSober),
            ksni::MenuItem::Separator,
            self.item("Quit", TrayCommand::Quit),
        ]
    }
}

pub struct TrayBridge {
    handle: Handle<AntiAFKTray>,
    queue: Arc<Mutex<Vec<TrayCommand>>>,
    last_running: Option<bool>,
}

impl TrayBridge {
    pub fn spawn(state: SharedState) -> Option<Self> {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let tray = AntiAFKTray {
            queue: queue.clone(),
            state,
        };
        let handle = tray.spawn().ok()?;
        Some(Self {
            handle,
            queue,
            last_running: None,
        })
    }

    pub fn drain(&self) -> Vec<TrayCommand> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }

    pub fn refresh(&mut self, running: bool) {
        if self.last_running == Some(running) {
            return;
        }
        self.last_running = Some(running);
        let _ = self.handle.update(|_| {});
    }
}
