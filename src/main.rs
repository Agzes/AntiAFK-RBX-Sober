mod backend;
mod input;
mod inputs;
mod state;
mod ui;

use clap::Parser;
use gtk::Application;
use gtk::prelude::*;
use image::GenericImageView;
use state::SharedState;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    tray: bool,
    #[arg(long)]
    install: bool,
}

struct AntiAFKTray {
    tx: glib::Sender<TrayCommand>,
    state: SharedState,
}

#[derive(Debug)]
enum TrayCommand {
    Show,
    Quit,
    ShowSober,
    HideSober,
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

static TRAY_OFF_ICON: std::sync::LazyLock<Vec<ksni::Icon>> =
    std::sync::LazyLock::new(|| load_icon(include_bytes!("../assets/tray-off.png")));
static TRAY_RUN_ICON: std::sync::LazyLock<Vec<ksni::Icon>> =
    std::sync::LazyLock::new(|| load_icon(include_bytes!("../assets/tray-run.png")));

impl ksni::Tray for AntiAFKTray {
    fn icon_name(&self) -> String {
        String::new()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let is_running = self.state.lock().unwrap().running;
        if is_running {
            TRAY_RUN_ICON.clone()
        } else {
            TRAY_OFF_ICON.clone()
        }
    }
    fn id(&self) -> String {
        "antiafk-rbx".into()
    }
    fn title(&self) -> String {
        "AntiAFK RBX".into()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let tx_show = self.tx.clone();
        let tx_quit = self.tx.clone();
        let tx_show_sober = self.tx.clone();
        let tx_hide_sober = self.tx.clone();
        use ksni::menu::StandardItem;
        vec![
            ksni::MenuItem::Standard(StandardItem {
                label: "Show Window".into(),
                activate: Box::new(move |_| {
                    let _ = tx_show.send(TrayCommand::Show);
                }),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(StandardItem {
                label: "Show Sober".into(),
                activate: Box::new(move |_| {
                    let _ = tx_show_sober.send(TrayCommand::ShowSober);
                }),
                ..Default::default()
            }),
            ksni::MenuItem::Standard(StandardItem {
                label: "Hide Sober".into(),
                activate: Box::new(move |_| {
                    let _ = tx_hide_sober.send(TrayCommand::HideSober);
                }),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }),
        ]
    }
}

fn install() -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let bin_dir = home.join(".local/bin");
    let apps_dir = home.join(".local/share/applications");
    let icons_dir = home.join(".local/share/icons");

    std::fs::create_dir_all(&bin_dir)?;
    std::fs::create_dir_all(&apps_dir)?;
    std::fs::create_dir_all(&icons_dir)?;

    let current_exe = std::env::current_exe()?;
    let target_exe = bin_dir.join("antiafk-rbx-sober");

    if !target_exe.exists() || !files_match(&current_exe, &target_exe) {
        std::fs::copy(&current_exe, &target_exe)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&target_exe)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&target_exe, perms)?;
        }
    }

    std::fs::write(
        icons_dir.join("dev.agzes.antiafk-rbx-sober.png"),
        include_bytes!("../assets/logo.png"),
    )?;

    let desktop_entry = format!(
        "[Desktop Entry]\n\
        Name=AntiAFK-RBX-Sober\n\
        Comment=AntiAFK application for Sober on Linux\n\
        Exec={} \n\
        Icon=dev.agzes.antiafk-rbx-sober\n\
        Terminal=false\n\
        Type=Application\n\
        Categories=Game;Utility;\n\
        StartupWMClass=dev.agzes.antiafk-rbx-sober\n\
        Keywords=Roblox;Sober;AntiAFK;\n",
        target_exe.display()
    );

    std::fs::write(
        apps_dir.join("dev.agzes.antiafk-rbx-sober.desktop"),
        &desktop_entry,
    )?;

    Ok(())
}

pub fn sync_binary() {
    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(_) => return,
    };

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let target_bin = home.join(".local/bin/antiafk-rbx-sober");

    if current_exe == target_bin {
        return;
    }

    if target_bin.exists() {
        if !files_match(&current_exe, &target_bin) {
            println!("Updating binary in .local/bin...");
            let _ = std::fs::copy(&current_exe, &target_bin);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &target_bin,
                    std::fs::Permissions::from_mode(0o755),
                );
            }
        }
    }
}

fn files_match(p1: &std::path::Path, p2: &std::path::Path) -> bool {
    let mut f1 = match std::fs::File::open(p1) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut f2 = match std::fs::File::open(p2) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let m1 = f1.metadata().ok();
    let m2 = f2.metadata().ok();

    if let (Some(m1), Some(m2)) = (m1, m2) {
        if m1.len() != m2.len() {
            return false;
        }
    }

    use std::io::Read;
    let mut b1 = [0; 8192];
    let mut b2 = [0; 8192];

    loop {
        let n1 = f1.read(&mut b1).unwrap_or(0);
        let n2 = f2.read(&mut b2).unwrap_or(0);

        if n1 != n2 {
            return false;
        }
        if n1 == 0 {
            break;
        }
        if b1[..n1] != b2[..n1] {
            return false;
        }
    }

    true
}

fn main() -> glib::ExitCode {
    sync_binary();
    let args = Args::parse();

    if args.install {
        if let Err(e) = install() {
            eprintln!("Error during installation: {}", e);
            return glib::ExitCode::from(1);
        }
        println!("Successfully installed desktop entry and icon.");
        return glib::ExitCode::from(0);
    }

    let state: SharedState = Arc::new(Mutex::new(state::AppState::load()));
    backend::start_backend(state.clone());
    let app = Application::builder().application_id(state::APP_ID).build();
    let state_ui = state.clone();
    let is_tray_arg = args.tray;

    app.connect_activate(move |obj| {
        if let Some(window) = obj.windows().first() {
            window.present();
            return;
        }

        let window = ui::build_ui(obj, state_ui.clone());
        if is_tray_arg {
            window.set_visible(false);
        }

        #[allow(deprecated)]
        let (tx, rx) = glib::MainContext::channel(glib::Priority::default());
        let tray = AntiAFKTray {
            tx,
            state: state_ui.clone(),
        };
        let service = ksni::TrayService::new(tray);
        let handle = service.handle();
        service.spawn();

        let state_tray_poll = state_ui.clone();
        let mut last_running = false;
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let is_running = state_tray_poll.lock().unwrap().running;
            if is_running != last_running {
                last_running = is_running;
                handle.update(|_| {});
            }
            glib::ControlFlow::Continue
        });

        let window_clone = window.clone();
        let state_ui_tray = state_ui.clone();
        rx.attach(None, move |cmd| {
            match cmd {
                TrayCommand::Show => window_clone.present(),
                TrayCommand::Quit => {
                    {
                        let mut s = state_ui_tray.lock().unwrap();
                        s.running = false;
                    }
                    window_clone.set_visible(false);
                    glib::timeout_add_local(std::time::Duration::from_millis(1200), move || {
                        std::process::exit(0);
                    });
                }
                TrayCommand::ShowSober => {
                    if backend::is_hyprland() {
                        let _ = std::process::Command::new("hyprctl")
                            .args(["dispatch", "movetoworkspace", "current", "class:sober"])
                            .output();
                        let _ = std::process::Command::new("hyprctl")
                            .args(["dispatch", "focuswindow", "class:sober"])
                            .output();
                    }
                }
                TrayCommand::HideSober => {
                    if backend::is_hyprland() {
                        let _ = std::process::Command::new("hyprctl")
                            .args([
                                "dispatch",
                                "movetoworkspacesilent",
                                "special",
                                "class:sober",
                            ])
                            .output();
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        window.connect_close_request(move |win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });
    });

    app.run()
}
