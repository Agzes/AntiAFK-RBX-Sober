mod backend;
mod environment;
mod input;
mod inputs;
mod state;
mod tray;
mod ui;

use clap::Parser;
use state::{APP_ID, APP_SLUG, APP_TITLE, SharedState};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

pub const GNOME_EXTENSION_UUID: &str = "antiafk-rbx-sober@agzes.github.io";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    tray: bool,
    #[arg(long)]
    install: bool,
    #[arg(long)]
    uninstall: bool,
    #[arg(long)]
    purge: bool,
    #[arg(long, alias = "desktop", value_name = "DESKTOP")]
    force_desktop: Option<String>,
    #[arg(long)]
    diagnose_cosmic: bool,
    #[arg(long)]
    install_gnome_helper: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopIntegrationStatus {
    Aur { package: String, helper: String },
    Installed { up_to_date: bool },
    NotInstalled,
}

pub fn detect_aur_package() -> Option<String> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Ok(output) = std::process::Command::new("pacman")
            .env("LC_ALL", "C")
            .args(["-Qo", &current_exe.to_string_lossy()])
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let words: Vec<&str> = stdout.split_whitespace().collect();
            if let Some(pos) = words.iter().position(|w| *w == "by")
                && let Some(pkg) = words.get(pos + 1)
            {
                return Some(pkg.to_string());
            }
            if stdout.contains("antiafk-rbx-sober") {
                return Some("antiafk-rbx-sober".to_string());
            }
        }

        if current_exe.starts_with("/usr")
            && let Ok(output) = std::process::Command::new("pacman")
                .env("LC_ALL", "C")
                .args(["-Q", "antiafk-rbx-sober"])
                .output()
            && output.status.success()
        {
            return Some("antiafk-rbx-sober".to_string());
        }
    }

    None
}

pub fn detect_aur_helper() -> String {
    for helper in ["yay", "paru"] {
        if std::process::Command::new(helper)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return helper.to_string();
        }
    }
    "yay".to_string()
}

pub fn check_desktop_integration() -> DesktopIntegrationStatus {
    if let Some(package) = detect_aur_package() {
        return DesktopIntegrationStatus::Aur {
            package,
            helper: detect_aur_helper(),
        };
    }

    let Some(home) = dirs::home_dir() else {
        return DesktopIntegrationStatus::NotInstalled;
    };

    let apps_dir = home.join(".local/share/applications");
    let desktop_file = apps_dir.join(format!("{APP_ID}.desktop"));
    let target_bin = home.join(".local/bin").join(APP_SLUG);

    if desktop_file.exists() && target_bin.exists() {
        if let Ok(current_exe) = std::env::current_exe() {
            if current_exe == target_bin || files_match(&current_exe, &target_bin) {
                DesktopIntegrationStatus::Installed { up_to_date: true }
            } else {
                DesktopIntegrationStatus::Installed { up_to_date: false }
            }
        } else {
            DesktopIntegrationStatus::Installed { up_to_date: true }
        }
    } else {
        DesktopIntegrationStatus::NotInstalled
    }
}

pub fn install_desktop_entry() -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Home directory not found".to_string())?;
    let bin_dir = home.join(".local/bin");
    let apps_dir = home.join(".local/share/applications");
    let icons_dir = home.join(".local/share/icons");
    let hicolor_dir = home.join(".local/share/icons/hicolor/128x128/apps");
    let pixmaps_dir = home.join(".local/share/pixmaps");

    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("cannot create {}: {e}", bin_dir.display()))?;
    std::fs::create_dir_all(&apps_dir)
        .map_err(|e| format!("cannot create {}: {e}", apps_dir.display()))?;
    std::fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("cannot create {}: {e}", icons_dir.display()))?;
    let _ = std::fs::create_dir_all(&hicolor_dir);
    let _ = std::fs::create_dir_all(&pixmaps_dir);

    let current_exe =
        std::env::current_exe().map_err(|e| format!("cannot determine current executable: {e}"))?;
    let target_exe = bin_dir.join(APP_SLUG);

    if current_exe != target_exe {
        std::fs::copy(&current_exe, &target_exe)
            .map_err(|e| format!("cannot copy binary to {}: {e}", target_exe.display()))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&target_exe) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&target_exe, perms);
        }
    }

    let icon_bytes = include_bytes!("../assets/logo.png");
    let icon_name = format!("{APP_ID}.png");
    let _ = std::fs::write(icons_dir.join(&icon_name), icon_bytes);
    let _ = std::fs::write(hicolor_dir.join(&icon_name), icon_bytes);
    let _ = std::fs::write(pixmaps_dir.join(&icon_name), icon_bytes);

    let desktop_entry = format!(
        "[Desktop Entry]\n\
        Name={APP_TITLE}\n\
        Comment=Keeps Sober active on Linux\n\
        Exec={}\n\
        Icon={APP_ID}\n\
        Terminal=false\n\
        Type=Application\n\
        Categories=Game;Utility;\n\
        StartupWMClass={APP_ID}\n\
        Keywords=Roblox;Sober;AntiAFK;\n",
        target_exe.display()
    );

    let desktop_path = apps_dir.join(format!("{APP_ID}.desktop"));
    std::fs::write(&desktop_path, desktop_entry)
        .map_err(|e| format!("cannot write {}: {e}", desktop_path.display()))?;

    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    Ok(())
}

pub fn uninstall_desktop_entry() -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Home directory not found".to_string())?;

    let apps_dir = home.join(".local/share/applications");
    let desktop_file = apps_dir.join(format!("{APP_ID}.desktop"));
    let target_bin = home.join(".local/bin").join(APP_SLUG);
    let icon_file = home
        .join(".local/share/icons")
        .join(format!("{APP_ID}.png"));
    let hicolor_icon = home
        .join(".local/share/icons/hicolor/128x128/apps")
        .join(format!("{APP_ID}.png"));
    let pixmap_icon = home
        .join(".local/share/pixmaps")
        .join(format!("{APP_ID}.png"));

    if desktop_file.exists() {
        let _ = std::fs::remove_file(&desktop_file);
    }
    if target_bin.exists() {
        let _ = std::fs::remove_file(&target_bin);
    }
    if icon_file.exists() {
        let _ = std::fs::remove_file(&icon_file);
    }
    if hicolor_icon.exists() {
        let _ = std::fs::remove_file(&hicolor_icon);
    }
    if pixmap_icon.exists() {
        let _ = std::fs::remove_file(&pixmap_icon);
    }

    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    Ok(())
}

pub fn purge_all_data() -> Result<(), String> {
    let _ = uninstall_desktop_entry();

    if let Some(home) = dirs::home_dir() {
        let ext_dir = home
            .join(".local/share/gnome-shell/extensions")
            .join(GNOME_EXTENSION_UUID);
        if ext_dir.exists() {
            let _ = std::process::Command::new("gnome-extensions")
                .args(["uninstall", GNOME_EXTENSION_UUID])
                .output();
            let _ = std::fs::remove_dir_all(&ext_dir);
        }
    }

    if let Some(config_dir) = dirs::config_dir() {
        let app_config = config_dir.join(APP_SLUG);
        if app_config.exists() {
            let _ = std::fs::remove_dir_all(&app_config);
        }
    }

    Ok(())
}

pub fn install_gnome_helper() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let target = home
        .join(".local/share/gnome-shell/extensions")
        .join(GNOME_EXTENSION_UUID);
    std::fs::create_dir_all(&target)
        .map_err(|error| format!("cannot create {}: {error}", target.display()))?;

    for (name, contents) in [
        (
            "metadata.json",
            include_bytes!("../assets/gnome-extension/metadata.json").as_slice(),
        ),
        (
            "extension.js",
            include_bytes!("../assets/gnome-extension/extension.js").as_slice(),
        ),
        (
            "README.md",
            include_bytes!("../assets/gnome-extension/README.md").as_slice(),
        ),
    ] {
        std::fs::write(target.join(name), contents)
            .map_err(|error| format!("cannot write {}: {error}", name))?;
    }

    let _ = std::process::Command::new("gnome-extensions")
        .args(["enable", GNOME_EXTENSION_UUID])
        .output();
    Ok(())
}

pub fn files_match(p1: &std::path::Path, p2: &std::path::Path) -> bool {
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

    if let (Some(m1), Some(m2)) = (m1, m2)
        && m1.len() != m2.len()
    {
        return false;
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
        if b1[..n1] != b2[..n2] {
            return false;
        }
    }

    true
}

fn apply_desktop_override(raw: Option<&str>) -> Result<(), String> {
    let requested = match raw {
        Some(value) => Some(value.to_string()),
        None => std::env::var("ANTIAFK_FORCE_DESKTOP").ok(),
    };
    let Some(requested) = requested else {
        return Ok(());
    };
    let trimmed = requested.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return Ok(());
    }
    match environment::InputMode::parse_force_name(trimmed)? {
        Some(mode) => {
            environment::force_desktop(Some(mode));
            Ok(())
        }
        None => Err(format!("unsupported desktop override: {requested}")),
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    if let Err(error) = apply_desktop_override(args.force_desktop.as_deref()) {
        eprintln!("Invalid desktop override: {error}");
        return ExitCode::from(1);
    }

    if args.diagnose_cosmic {
        return match inputs::cosmic::probe() {
            Ok(()) => {
                println!("COSMIC protocol probe: OK");
                ExitCode::SUCCESS
            }
            Err(error) => {
                println!("COSMIC protocol probe: {error}");
                ExitCode::from(1)
            }
        };
    }

    if args.install {
        if let Err(e) = install_desktop_entry() {
            eprintln!("Error during installation: {e}");
            return ExitCode::from(1);
        }
        println!("Successfully installed desktop entry and icon.");
        return ExitCode::SUCCESS;
    }

    if args.uninstall {
        if let Err(e) = uninstall_desktop_entry() {
            eprintln!("Error during uninstallation: {e}");
            return ExitCode::from(1);
        }
        println!("Successfully removed desktop entry, icons, and binary.");
        return ExitCode::SUCCESS;
    }

    if args.purge {
        if let Err(e) = purge_all_data() {
            eprintln!("Error during purge: {e}");
            return ExitCode::from(1);
        }
        println!("Successfully purged all configuration files, desktop entries, and extensions.");
        return ExitCode::SUCCESS;
    }

    if args.install_gnome_helper {
        if let Err(error) = install_gnome_helper() {
            eprintln!("Error during GNOME helper installation: {error}");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    let state: SharedState = Arc::new(Mutex::new(state::AppState::load()));
    backend::start_backend(state.clone());

    match ui::run(state, args.tray) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("UI error: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_files_match() {
        let dir = std::env::temp_dir();
        let f1 = dir.join("test_antiafk_1.tmp");
        let f2 = dir.join("test_antiafk_2.tmp");
        let f3 = dir.join("test_antiafk_3.tmp");

        std::fs::write(&f1, b"hello world 12345").unwrap();
        std::fs::write(&f2, b"hello world 12345").unwrap();
        std::fs::write(&f3, b"different content").unwrap();

        assert!(files_match(&f1, &f2));
        assert!(!files_match(&f1, &f3));

        let _ = std::fs::remove_file(f1);
        let _ = std::fs::remove_file(f2);
        let _ = std::fs::remove_file(f3);
    }

    #[test]
    fn test_aur_package_parsing() {
        let stdout = "/usr/bin/AntiAFK-RBX-Sober is owned by antiafk-rbx-sober 0.2.0-1\n";
        let words: Vec<&str> = stdout.split_whitespace().collect();
        let pos = words.iter().position(|w| *w == "by");
        assert_eq!(pos, Some(3));
        assert_eq!(words.get(pos.unwrap() + 1), Some(&"antiafk-rbx-sober"));
    }

    #[test]
    fn test_uninstall_and_purge_do_not_fail_when_clean() {
        assert!(uninstall_desktop_entry().is_ok());
        assert!(purge_all_data().is_ok());
    }
}
