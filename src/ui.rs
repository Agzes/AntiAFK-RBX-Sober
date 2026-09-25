use crate::state::{RuntimeStatus, SharedState, set_runtime_error, set_runtime_status};
use gtk::gdk_pixbuf::PixbufLoader;
use gtk::gio;
use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Application, ApplicationWindow, Box, Button, CssProvider, Image, Label,
    ListBox, ListBoxRow, Orientation, Overlay, Popover, Revealer, ScrolledWindow, SpinButton,
    Stack, Switch,
};
use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;
use std::sync::OnceLock;

const CURRENT_VERSION: &str = "0.2.0";

const CSS: &str = "
    .main-window { background-color: @theme_bg_color; color: @theme_fg_color; }
    .main-box { padding: 0; }
    .header-box { min-height: 36px; margin: 0; padding: 4px 12px; border-bottom: 1px solid alpha(@theme_fg_color, 0.08); }
    .page-content { margin: 8px 16px 12px; }
    .app-title { font-size: 24px; font-weight: 800; letter-spacing: -0.5px; }
    .compact-title { font-size: 15px; font-weight: 750; }
    .app-subtitle { font-size: 13px; opacity: 0.72; }
    .version-btn { font-size: 9px; font-weight: bold; padding: 1px 5px; border-radius: 6px; background-color: alpha(@theme_fg_color, 0.08); color: @theme_fg_color; border: 1px solid alpha(@theme_fg_color, 0.12); margin-left: 8px; min-height: 18px; }
    .version-btn:hover { background-color: alpha(@theme_fg_color, 0.15); border: 1px solid alpha(@theme_fg_color, 0.2); }
    .top-icon-button { padding: 0; width: 28px; height: 28px; min-width: 28px; min-height: 28px; border-radius: 8px; background-color: alpha(@theme_fg_color, 0.07); border: none; }
    .top-icon-button:hover { background-color: alpha(@theme_fg_color, 0.12); }
    .section-title { font-size: 12px; font-weight: 700; opacity: 0.78; }
    .card { background-color: alpha(@theme_fg_color, 0.04); border: 1px solid alpha(@theme_fg_color, 0.08); border-radius: 12px; }
    list { background-color: transparent; border-radius: 12px; }
    row { padding: 8px 14px; border-bottom: 1px solid alpha(@theme_fg_color, 0.05); }
    row:first-child { border-top-left-radius: 12px; border-top-right-radius: 12px; }
    row:last-child { border-bottom-left-radius: 12px; border-bottom-right-radius: 12px; border-bottom: none; }
    row label.row-title { font-weight: 500; font-size: 13px; }
    .row-title-adjust { margin-top: 0.5px; }
    row label.row-subtitle { font-size: 11px; opacity: 0.5; }
    row.sub-row > box { margin-left: 20px; opacity: 0.85; }
    row.sub-row label.row-title { font-size: 13px; }
    .info-icon { opacity: 0.4; }
    .info-icon:hover { opacity: 0.9; }
    popover contents { background-color: @theme_bg_color; border: 1px solid alpha(@theme_fg_color, 0.15); border-radius: 12px; padding: 6px; color: @theme_fg_color; box-shadow: none; }
    popover listview, popover list { background-color: transparent; }
    popover listitem, popover row { padding: 8px 12px; border-radius: 10px; margin: 2px; transition: all 150ms ease; }
    popover listitem:hover, popover row:hover { background-color: alpha(@theme_fg_color, 0.08); }
    popover listitem:selected, popover row:selected { background-color: alpha(@theme_selected_bg_color, 0.35); color: @theme_fg_color; }
    spinbutton { min-height: 26px; font-size: 12px; border-radius: 6px; padding: 0; background-color: alpha(@theme_fg_color, 0.05); border: 1px solid alpha(@theme_fg_color, 0.1); }
    spinbutton button { background: none; border: none; padding: 0 4px; min-height: 22px; box-shadow: none; }
    spinbutton button:hover { background-color: alpha(@theme_fg_color, 0.05); }
    switch { margin: 0; outline: none; }
    .start-button, .stop-button { min-height: 40px; border-radius: 10px; padding: 0 24px; font-weight: 600; font-size: 14px; border: none; box-shadow: none; margin-bottom: 0; color: @theme_selected_fg_color; }
    .start-button { background-color: @theme_selected_bg_color; }
    .stop-button { background-color: @error_color; }
    .start-button:hover, .stop-button:hover { opacity: 0.88; }
    .start-button:active, .stop-button:active { opacity: 0.76; }
    .status-text { font-size: 11px; opacity: 0.7; }
    .status-text.active { color: @success_color; opacity: 1; }
    .status-text.paused { color: @warning_color; opacity: 1; }
    .status-text.error { color: @error_color; opacity: 1; }
    .settings-list { background-color: transparent; border: none; }
    .main-settings-list { margin-top: -4px; margin-bottom: -4px; margin-left: -10px; margin-right: -10px; }
    .diagnostics-list { margin-top: -4px; margin-bottom: -4px; margin-left: -10px; margin-right: -10px; }
    .settings-list > row { min-height: 44px; padding: 0 8px; border-bottom: 1px solid alpha(@theme_fg_color, 0.08); }
    .settings-list > row:last-child { border-bottom: none; }
    .settings-row-icon { opacity: 0.78; }
    .bottom-bar { padding: 16px; border-top: 1px solid alpha(@theme_fg_color, 0.1); }
    .compact-select { min-width: 0; min-height: 30px; height: 30px; padding: 0 7px; border-radius: 7px; font-size: 12px; background-color: @theme_base_color; border: 1px solid alpha(@theme_fg_color, 0.14); color: @theme_fg_color; box-shadow: none; }
    .compact-select:hover { background-color: alpha(@theme_fg_color, 0.1); }
    .compact-select label { font-size: 12px; }
    .select-option { min-height: 32px; padding: 5px 10px; border: none; border-radius: 7px; background-color: transparent; }
    .select-option:hover { background-color: alpha(@theme_fg_color, 0.08); }
    .status-indicator { padding: 5px 10px; border-radius: 15px; background-color: @theme_base_color; border: none; }
    .diagnostics-button { border-radius: 7px; padding: 6px 10px; font-size: 11px; }
    .back-button { min-height: 40px; border-radius: 10px; padding: 0 24px; font-weight: 600; font-size: 14px; background-color: alpha(@theme_fg_color, 0.08); border: none; color: @theme_fg_color; }
    .back-button:hover { background-color: alpha(@theme_fg_color, 0.13); }
    .diagnostic-ok { color: @success_color; font-size: 12px; font-weight: 600; }
    .diagnostic-error { color: @error_color; font-size: 12px; font-weight: 600; }
    .diagnostic-warning { color: @warning_color; font-size: 12px; font-weight: 600; }
    .welcome-title { font-size: 32px; font-weight: 800; letter-spacing: -1px; }
    .welcome-subtitle { font-size: 16px; margin-bottom: 20px; }
    .hypr-badge { background-color: alpha(@error_color, 0.2); color: @error_color; padding: 6px 14px; border-radius: 99px; font-size: 11px; font-weight: 800; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px; }
    .info-note { background-color: alpha(@theme_fg_color, 0.03); border: 1px solid alpha(@theme_fg_color, 0.07); padding: 20px; border-radius: 16px; margin: 10px 0; }
    .rbx-text { color: @error_color; font-weight: 800; }
    .sober-text { color: @theme_selected_bg_color; }
    .header-separator { font-size: 15px; font-weight: 500; opacity: 0.65; margin-left: 2px; margin-right: 2px; }
    .sober-edition { font-size: 14px; font-weight: 600; }
    .error-text { color: @error_color; }
    .success-text { color: @success_color; }
    .warning-text { color: @warning_color; }
    .small-text { font-size: 11px; }

";

static DARK_THEME: OnceLock<bool> = OnceLock::new();

fn system_uses_dark_theme() -> bool {
    if let Ok(theme) = std::env::var("GTK_THEME")
        && theme.to_lowercase().contains("dark")
    {
        return true;
    }

    for (schema, key) in [
        ("org.gnome.desktop.interface", "color-scheme"),
        ("org.gnome.desktop.interface", "gtk-theme"),
        ("org.kde.gtk.config", "darkMode"),
    ] {
        let Ok(output) = Command::new("gsettings")
            .args(["get", schema, key])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let value = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if value.contains("prefer-dark") || value.contains("dark") || value.trim() == "true" {
            return true;
        }
    }

    false
}

fn bundled_icon(name: &str, pixel_size: i32) -> Image {
    let dark_theme = *DARK_THEME.get_or_init(system_uses_dark_theme);
    macro_rules! themed_icon {
        ($icon:literal) => {{
            let dark = include_bytes!(concat!("../assets/icons/dark/", $icon, ".png"));
            let light = include_bytes!(concat!("../assets/icons/light/", $icon, ".png"));
            if dark_theme {
                dark.as_slice()
            } else {
                light.as_slice()
            }
        }};
    }

    let data = match name {
        "interval" => themed_icon!("interval"),
        "action" => themed_icon!("action"),
        "autostart" => themed_icon!("autostart"),
        "mouse" => themed_icon!("mouse"),
        "multi" => themed_icon!("multi"),
        "hide" => themed_icon!("hide"),
        "reconnect" => themed_icon!("reconnect"),
        "performance" => themed_icon!("performance"),
        "cpu" => themed_icon!("cpu"),
        "focus" => themed_icon!("focus"),
        "settings" => themed_icon!("settings"),
        "info" => themed_icon!("info"),
        "check" => themed_icon!("check"),
        "warning" => themed_icon!("warning"),
        "chevron-down" => themed_icon!("chevron-down"),
        "repository" => themed_icon!("repository"),
        _ => return bundled_icon("warning", pixel_size),
    };

    let loader = PixbufLoader::new();
    let _ = loader.write(data);
    let _ = loader.close();
    let image = Image::builder().pixel_size(pixel_size).build();
    if let Some(pixbuf) = loader.pixbuf() {
        image.set_from_pixbuf(Some(&pixbuf));
    }
    image
}

fn create_compact_select(
    initial_index: usize,
    options: &[&str],
) -> (Button, Popover, Rc<Label>, Rc<Cell<usize>>, Rc<Cell<bool>>) {
    let initial = options.get(initial_index).copied().unwrap_or(options[0]);
    let selected_label = Rc::new(Label::new(Some(initial)));
    selected_label.set_halign(Align::Start);
    selected_label.set_hexpand(true);
    let content = Box::new(Orientation::Horizontal, 5);
    content.set_hexpand(true);
    content.add_css_class("compact-select-content");
    content.append(selected_label.as_ref());
    content.append(&bundled_icon("chevron-down", 10));

    let button = Button::new();
    button.add_css_class("compact-select");
    button.set_halign(Align::End);
    button.set_hexpand(false);
    button.set_child(Some(&content));

    let popover = Popover::new();
    popover.set_parent(&button);
    popover.set_has_arrow(false);
    let menu = Box::new(Orientation::Vertical, 2);
    menu.set_margin_top(4);
    menu.set_margin_bottom(4);
    menu.set_margin_start(4);
    menu.set_margin_end(4);

    let selected_index = Rc::new(Cell::new(initial_index));
    let selection_changed = Rc::new(Cell::new(false));
    for (index, option) in options.iter().enumerate() {
        let option = option.to_string();
        let option_button = Button::with_label(&option);
        option_button.add_css_class("select-option");
        let label = selected_label.clone();
        let popover_for_click = popover.clone();
        let selected_for_click = selected_index.clone();
        let changed_for_click = selection_changed.clone();
        option_button.connect_clicked(move |_| {
            label.set_text(&option);
            selected_for_click.set(index);
            changed_for_click.set(true);
            popover_for_click.popdown();
        });
        menu.append(&option_button);
    }
    popover.set_child(Some(&menu));

    let popover_for_button = popover.clone();
    button.connect_clicked(move |_| {
        popover_for_button.popup();
    });
    button.set_cursor_from_name(Some("pointer"));

    (
        button,
        popover,
        selected_label,
        selected_index,
        selection_changed,
    )
}

fn check_uinput_permission() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok()
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn create_settings_row(
    icon_name: &str,
    title: &str,
    subtitle: Option<&str>,
    widget: &impl IsA<gtk::Widget>,
    info_text: Option<&str>,
    is_sub: bool,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_height_request(44);
    row.add_css_class("settings-row");
    if is_sub {
        row.add_css_class("sub-row");
    }

    let content = Box::new(Orientation::Horizontal, 8);
    content.set_valign(Align::Center);
    let icon = bundled_icon(icon_name, 18);
    icon.add_css_class("settings-row-icon");
    icon.set_valign(Align::Center);
    icon.set_margin_bottom(2);
    content.append(&icon);

    let text = Box::new(Orientation::Vertical, 0);
    if subtitle.is_none() {
        text.set_size_request(-1, 20);
    }
    text.set_valign(Align::Center);
    let title_label = Label::builder()
        .label(title)
        .halign(Align::Start)
        .css_classes(["row-title"])
        .build();
    if !is_sub {
        title_label.add_css_class("row-title-adjust");
    }
    text.append(&title_label);
    if let Some(subtitle) = subtitle {
        text.append(
            &Label::builder()
                .label(subtitle)
                .halign(Align::Start)
                .css_classes(["row-subtitle"])
                .build(),
        );
    }
    content.append(&text);

    let filler = Box::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    content.append(&filler);

    if let Some(info_text) = info_text {
        let info = bundled_icon("info", 16);
        info.add_css_class("info-icon");
        info.set_tooltip_text(Some(info_text));
        info.set_valign(Align::Center);
        content.append(&info);
    }

    widget.set_valign(Align::Center);
    content.append(widget);
    row.set_child(Some(&content));
    row.set_activatable(false);
    row.set_selectable(false);
    row
}

pub fn build_ui(app: &Application, state: SharedState) -> ApplicationWindow {
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Display error"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let window = ApplicationWindow::builder()
        .application(app)
        .title("AntiAFK-RBX")
        .default_width(440)
        .default_height(520)
        .resizable(true)
        .build();
    window.add_css_class("main-window");
    window.set_size_request(320, 420);

    let root_vbox = Box::new(Orientation::Vertical, 0);
    root_vbox.add_css_class("main-box");
    window.set_child(Some(&root_vbox));

    let header_box = Box::new(Orientation::Horizontal, 0);
    header_box.add_css_class("header-box");

    let title_hbox = Box::new(Orientation::Horizontal, 0);
    title_hbox.set_hexpand(true);
    title_hbox.set_halign(Align::Start);
    title_hbox.set_valign(Align::Center);
    title_hbox.append(
        &Label::builder()
            .label("AntiAFK-")
            .valign(Align::Center)
            .css_classes(["compact-title"])
            .build(),
    );
    title_hbox.append(
        &Label::builder()
            .label("RBX")
            .valign(Align::Center)
            .css_classes(["compact-title", "rbx-text"])
            .build(),
    );
    title_hbox.append(
        &Label::builder()
            .label("-")
            .valign(Align::Center)
            .css_classes(["compact-title"])
            .build(),
    );
    title_hbox.append(
        &Label::builder()
            .label("Sober")
            .valign(Align::Center)
            .css_classes(["compact-title", "sober-text"])
            .build(),
    );
    title_hbox.set_margin_bottom(1);

    let status_indicator = Box::new(Orientation::Horizontal, 0);
    status_indicator.set_valign(Align::Center);
    status_indicator.add_css_class("status-indicator");
    let status_line = Label::builder()
        .label("")
        .halign(Align::Center)
        .css_classes(["status-text"])
        .valign(Align::Center)
        .build();
    status_indicator.append(&status_line);
    let status_revealer = Revealer::new();
    status_revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    status_revealer.set_transition_duration(180);
    status_revealer.set_reveal_child(false);
    status_revealer.set_child(Some(&status_indicator));
    status_revealer.set_size_request(-1, 36);
    status_revealer.set_halign(Align::Center);
    status_revealer.set_valign(Align::Center);

    let title_overlay = Overlay::new();
    title_overlay.set_hexpand(true);
    title_overlay.set_halign(Align::Fill);
    title_overlay.set_valign(Align::Center);
    title_overlay.set_child(Some(&title_hbox));
    title_overlay.add_overlay(&status_revealer);
    header_box.append(&title_overlay);

    let repository_icon = bundled_icon("repository", 16);
    let repository_btn = Button::builder()
        .css_classes(["top-icon-button"])
        .valign(Align::Center)
        .has_frame(false)
        .tooltip_text("Open GitHub repository")
        .build();
    repository_btn.set_halign(Align::Center);
    repository_btn.set_valign(Align::Center);
    repository_btn.set_child(Some(&repository_icon));
    repository_btn.connect_clicked(|_| {
        let _ = Command::new("xdg-open")
            .arg("https://github.com/Agzes/AntiAFK-RBX-Sober")
            .spawn();
    });

    let settings_icon = bundled_icon("settings", 16);
    let combined_btn = Button::builder()
        .css_classes(["top-icon-button"])
        .valign(Align::Center)
        .has_frame(false)
        .tooltip_text("Diagnostics")
        .build();
    combined_btn.set_halign(Align::Center);
    combined_btn.set_valign(Align::Center);
    combined_btn.set_child(Some(&settings_icon));
    let header_right = Box::new(Orientation::Horizontal, 4);
    header_right.set_size_request(60, 28);
    header_right.set_halign(Align::Center);
    header_right.set_valign(Align::Center);
    header_right.append(&repository_btn);
    header_right.append(&combined_btn);
    header_box.append(&header_right);
    root_vbox.append(&header_box);

    let stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .build();

    let main_vbox = Box::new(Orientation::Vertical, 0);
    main_vbox.add_css_class("page-content");
    let compat_vbox = Box::new(Orientation::Vertical, 0);
    compat_vbox.add_css_class("page-content");
    compat_vbox.set_vexpand(true);

    let warning_vbox = Box::new(Orientation::Vertical, 0);
    warning_vbox.add_css_class("page-content");
    warning_vbox.set_vexpand(true);

    warning_vbox.append(&Box::builder().height_request(80).build());

    let welcome_icon = bundled_icon("warning", 80);
    welcome_icon.set_margin_bottom(20);
    welcome_icon.set_halign(Align::Center);
    warning_vbox.append(&welcome_icon);

    let w_title_hbox = Box::new(Orientation::Horizontal, 0);
    w_title_hbox.set_halign(Align::Center);
    w_title_hbox.append(
        &Label::builder()
            .label("AntiAFK-")
            .css_classes(["welcome-title"])
            .build(),
    );
    w_title_hbox.append(
        &Label::builder()
            .label("RBX")
            .css_classes(["welcome-title", "rbx-text"])
            .build(),
    );
    warning_vbox.append(&w_title_hbox);

    let w_sub = Label::builder()
        .use_markup(true)
        .label("<b>Sober Edition</b>")
        .css_classes(["welcome-subtitle", "sober-text"])
        .halign(Align::Center)
        .build();
    warning_vbox.append(&w_sub);

    let note_box = Box::new(Orientation::Vertical, 8);
    note_box.add_css_class("info-note");
    note_box.set_halign(Align::Center);

    let note_text = Label::builder()
        .use_markup(true)
        .label(
            "<span size='large' weight='800'>Wayland Only - WIP</span>\n\n\
        This project is currently a <b>Work In Progress</b>.\n\
        It works on <b>Hyprland</b> and <b>KDE Plasma 6 (Wayland)</b>.\n\
        GNOME, X11 and other are <b>not</b> supported yet.",
        )
        .justify(gtk::Justification::Center)
        .build();
    note_box.append(&note_text);
    warning_vbox.append(&note_box);

    let filler = Box::new(Orientation::Vertical, 0);
    filler.set_vexpand(true);
    warning_vbox.append(&filler);

    let ok_btn = Button::builder()
        .label("Get Started")
        .css_classes(["start-button"])
        .margin_bottom(10)
        .build();
    let stack_warn_clone = stack.clone();
    let state_warn_clone = state.clone();
    ok_btn.connect_clicked(move |_| {
        let mut s = state_warn_clone.lock().unwrap();
        s.shown_warning = true;
        s.save();
        stack_warn_clone.set_visible_child_name("main");
    });
    warning_vbox.append(&ok_btn);

    stack.add_named(&main_vbox, Some("main"));
    stack.add_named(&compat_vbox, Some("compat"));
    stack.add_named(&warning_vbox, Some("warning"));

    let stack_scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    stack_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    stack_scroller.set_child(Some(&stack));
    root_vbox.append(&stack_scroller);

    let bottom_bar = Overlay::new();
    bottom_bar.add_css_class("bottom-bar");
    root_vbox.append(&bottom_bar);

    let version_check_widgets = create_version_check_widgets();
    let compat_vbox_clone = compat_vbox.clone();
    let stack_clone = stack.clone();
    let state_clone = state.clone();
    let version_widgets_clone = version_check_widgets.clone();
    let refresh_compat = move || {
        while let Some(child) = compat_vbox_clone.first_child() {
            compat_vbox_clone.remove(&child);
        }
        build_compat_ui(
            compat_vbox_clone.clone(),
            stack_clone.clone(),
            state_clone.clone(),
            version_widgets_clone.clone(),
        );
        stack_clone.set_visible_child_name("compat");
    };

    let rc = refresh_compat.clone();
    let stack_for_settings = stack.clone();
    combined_btn.connect_clicked(move |_| {
        if stack_for_settings
            .visible_child_name()
            .is_some_and(|name| name == "compat")
        {
            stack_for_settings.set_visible_child_name("main");
        } else {
            rc();
        }
    });

    {
        let mut state = state.lock().unwrap();
        state.shown_warning = true;
    }
    stack.set_visible_child_name("main");

    let btn_container = Box::new(Orientation::Vertical, 4);
    btn_container.set_hexpand(true);
    btn_container.set_halign(Align::Fill);
    bottom_bar.set_child(Some(&btn_container));

    let toggle_button = Button::builder().label("Start Anti-AFK").build();
    toggle_button.set_hexpand(true);
    toggle_button.set_halign(Align::Fill);
    toggle_button.add_css_class("start-button");

    let runtime_error = Label::builder()
        .halign(Align::Start)
        .wrap(true)
        .margin_top(4)
        .visible(false)
        .css_classes(["error-text", "small-text"])
        .build();
    btn_container.append(&runtime_error);

    let diagnostics_button = Button::builder()
        .label("Open diagnostics")
        .halign(Align::Start)
        .visible(false)
        .css_classes(["diagnostics-button"])
        .build();
    let diagnostics_refresh = refresh_compat.clone();
    diagnostics_button.connect_clicked(move |_| diagnostics_refresh());
    btn_container.append(&diagnostics_button);
    btn_container.append(&toggle_button);

    let initial_state = { state.lock().unwrap().clone() };

    let settings_list = ListBox::new();
    settings_list.add_css_class("settings-list");
    settings_list.add_css_class("main-settings-list");

    let interval_index = match initial_state.interval_seq {
        240 => 0,
        360 => 1,
        540 => 2,
        1140 => 3,
        _ => 4,
    };
    let (interval_select, interval_popover, interval_label, interval_selected, interval_changed) =
        create_compact_select(
            interval_index,
            &["4 min", "6 min", "9 min", "19 min", "Custom"],
        );
    interval_select.set_size_request(96, 30);
    settings_list.append(&create_settings_row(
        "interval",
        "Interval",
        None,
        &interval_select,
        None,
        false,
    ));

    let action_index = if initial_state.walk {
        1
    } else if initial_state.spin_jiggle {
        2
    } else {
        0
    };
    let (action_select, action_popover, _action_label, action_selected, action_changed) =
        create_compact_select(action_index, &["Jump", "Walk", "Camera"]);
    action_select.set_size_request(82, 30);
    settings_list.append(&create_settings_row(
        "action",
        "Idle action",
        None,
        &action_select,
        None,
        false,
    ));
    main_vbox.append(&settings_list);

    let custom_interval_adjustment = Adjustment::new(
        (initial_state.interval_seq as f64 / 60.0).clamp(1.0, 20.0),
        1.0,
        20.0,
        1.0,
        1.0,
        0.0,
    );
    let custom_interval_spin = SpinButton::builder()
        .adjustment(&custom_interval_adjustment)
        .climb_rate(1.0)
        .digits(1)
        .numeric(true)
        .build();
    let custom_popover = Popover::new();
    custom_popover.set_parent(&interval_select);
    custom_popover.set_has_arrow(false);
    let custom_popover_box = Box::new(Orientation::Vertical, 8);
    custom_popover_box.set_margin_top(12);
    custom_popover_box.set_margin_bottom(12);
    custom_popover_box.set_margin_start(12);
    custom_popover_box.set_margin_end(12);
    custom_popover_box.append(
        &Label::builder()
            .label("Custom interval (minutes)")
            .css_classes(["section-title"])
            .build(),
    );
    custom_popover_box.append(&custom_interval_spin);
    let custom_apply_button = Button::builder().label("Apply").halign(Align::End).build();
    custom_popover_box.append(&custom_apply_button);
    custom_popover.set_child(Some(&custom_popover_box));
    {
        let state_for_interval = state.clone();
        let interval_label = interval_label.clone();
        let interval_selected = interval_selected.clone();
        custom_popover.connect_closed(move |_| {
            let interval = state_for_interval.lock().unwrap().interval_seq;
            let (selected, label) = match interval {
                240 => (0, "4 min"),
                360 => (1, "6 min"),
                540 => (2, "9 min"),
                1140 => (3, "19 min"),
                _ => (4, "Custom"),
            };
            interval_selected.set(selected);
            interval_label.set_text(label);
        });
    }
    let multi_instance_sw = Switch::new();
    multi_instance_sw.set_active(initial_state.multi_instance);
    settings_list.append(&create_settings_row(
        "multi",
        "Multi-Instance",
        None,
        &multi_instance_sw,
        None,
        false,
    ));

    let auto_start_sw = Switch::new();
    auto_start_sw.set_active(initial_state.auto_start);
    settings_list.append(&create_settings_row(
        "autostart",
        "Auto-Start",
        None,
        &auto_start_sw,
        None,
        false,
    ));
    let user_safe_sw = Switch::new();
    user_safe_sw.set_active(initial_state.user_safe);
    settings_list.append(&create_settings_row(
        "mouse",
        "Don't Interrupt Me",
        None,
        &user_safe_sw,
        Some("Current desktop adapters detect mouse movement only."),
        false,
    ));

    let stealth_sw = Switch::new();
    stealth_sw.set_active(initial_state.stealth);
    settings_list.append(&create_settings_row(
        "hide",
        "Hide Game",
        None,
        &stealth_sw,
        None,
        false,
    ));
    let reconnect_sw = Switch::new();
    reconnect_sw.set_active(initial_state.auto_reconnect);
    settings_list.append(&create_settings_row(
        "reconnect",
        "Auto Reconnect",
        None,
        &reconnect_sw,
        None,
        false,
    ));

    let fps_capper_sw = Switch::new();
    fps_capper_sw.set_active(initial_state.fps_capper);
    settings_list.append(&create_settings_row(
        "performance",
        "FPS Capper",
        None,
        &fps_capper_sw,
        None,
        false,
    ));
    let fps_adj = Adjustment::new(
        f64::from(initial_state.fps_limit),
        3.0,
        99.0,
        1.0,
        10.0,
        0.0,
    );
    let fps_limit_spin = SpinButton::builder()
        .adjustment(&fps_adj)
        .climb_rate(1.0)
        .digits(0)
        .numeric(true)
        .build();
    let fps_limit_row = create_settings_row("cpu", "CPU Quota", None, &fps_limit_spin, None, true);
    fps_limit_row.set_visible(initial_state.fps_capper);
    settings_list.append(&fps_limit_row);
    let fps_limit_widget = fps_limit_spin.clone().upcast::<gtk::Widget>();
    let unlock_focus_sw = Switch::new();
    unlock_focus_sw.set_active(initial_state.stop_limit_on_focus);
    let unlock_focus_row = create_settings_row(
        "focus",
        "Unlock at Focus",
        None,
        &unlock_focus_sw,
        None,
        true,
    );
    unlock_focus_row.set_visible(initial_state.fps_capper);
    settings_list.append(&unlock_focus_row);
    let unlock_focus_widget = unlock_focus_sw.clone().upcast::<gtk::Widget>();

    let fps_limit_row_clone = fps_limit_row.clone();
    let unlock_focus_row_clone = unlock_focus_row.clone();
    let fps_limit_widget_clone = fps_limit_widget.clone();
    let unlock_focus_widget_clone = unlock_focus_widget.clone();
    let state_capper_sync = state.clone();
    fps_capper_sw.connect_state_set(move |_sw, state_val| {
        fps_limit_row_clone.set_visible(state_val);
        unlock_focus_row_clone.set_visible(state_val);
        let is_running = state_capper_sync.lock().unwrap().running;
        fps_limit_widget_clone.set_sensitive(!is_running);
        unlock_focus_widget_clone.set_sensitive(!is_running);
        glib::Propagation::Proceed
    });

    let stealth_live = stealth_sw.clone();
    let update_state = {
        let state_arc = state.clone();
        let auto_start_live = auto_start_sw.clone();
        let multi_instance_live = multi_instance_sw.clone();
        let stealth_live = stealth_live.clone();
        let user_safe_live = user_safe_sw.clone();
        let auto_reconnect_live = reconnect_sw.clone();
        let fps_capper_live = fps_capper_sw.clone();
        let fps_limit_live = fps_limit_spin.clone();
        let stop_limit_on_focus_live = unlock_focus_sw.clone();

        move || {
            let mut s = state_arc.lock().unwrap();
            if auto_start_live.is_active() && !s.auto_start {
                s.manually_stopped = false;
            }
            s.auto_start = auto_start_live.is_active();
            s.multi_instance = multi_instance_live.is_active();
            s.stealth = stealth_live.is_active();
            s.user_safe = user_safe_live.is_active();
            s.auto_reconnect = auto_reconnect_live.is_active();
            s.fps_capper = fps_capper_live.is_active();
            s.fps_limit = fps_limit_live.value() as u32;
            s.stop_limit_on_focus = stop_limit_on_focus_live.is_active();
            s.save();
        }
    };

    {
        let state_for_action = state.clone();
        let selected = action_selected.clone();
        let changed = action_changed.clone();
        action_popover.connect_closed(move |_| {
            if !changed.replace(false) {
                return;
            }
            let selected = selected.get();
            let mut state = state_for_action.lock().unwrap();
            state.jump = selected == 0;
            state.walk = selected == 1;
            state.spin_jiggle = selected == 2;
            state.save();
        });
    }
    {
        let state_for_interval = state.clone();
        let selected = interval_selected.clone();
        let changed = interval_changed.clone();
        let custom_popover = custom_popover.clone();
        let custom_interval_spin = custom_interval_spin.clone();
        interval_popover.connect_closed(move |_| {
            if !changed.replace(false) {
                return;
            }
            let selected = selected.get();
            if selected < 4 {
                let interval = [240, 360, 540, 1140][selected];
                let mut state = state_for_interval.lock().unwrap();
                state.interval_seq = interval;
                state.save();
            } else {
                let current = state_for_interval.lock().unwrap().interval_seq;
                custom_interval_spin.set_value((current as f64 / 60.0).clamp(1.0, 20.0));
                custom_popover.popup();
            }
        });
    }
    {
        let state_for_interval = state.clone();
        let popover = custom_popover.clone();
        let spin = custom_interval_spin.clone();
        custom_apply_button.connect_clicked(move |_| {
            let value = (spin.value() * 60.0).round() as u64;
            {
                let mut state = state_for_interval.lock().unwrap();
                state.interval_seq = value;
                state.save();
            }
            popover.popdown();
        });
    }

    let us = update_state.clone();
    auto_start_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });
    let us = update_state.clone();
    multi_instance_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });

    let us = update_state.clone();
    stealth_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });
    let us = update_state.clone();
    user_safe_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });
    let us = update_state.clone();
    reconnect_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });
    let us = update_state.clone();
    fps_capper_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });
    let us = update_state.clone();
    fps_limit_spin.connect_value_changed(move |_| us());
    let us = update_state.clone();
    unlock_focus_sw.connect_state_set(move |_, _| {
        us();
        glib::Propagation::Proceed
    });

    let controls: Vec<gtk::Widget> = vec![
        action_select.clone().upcast::<gtk::Widget>(),
        interval_select.clone().upcast::<gtk::Widget>(),
        custom_interval_spin.clone().upcast::<gtk::Widget>(),
        auto_start_sw.clone().upcast::<gtk::Widget>(),
        multi_instance_sw.clone().upcast::<gtk::Widget>(),
        stealth_sw.clone().upcast::<gtk::Widget>(),
        user_safe_sw.clone().upcast::<gtk::Widget>(),
        reconnect_sw.clone().upcast::<gtk::Widget>(),
        fps_capper_sw.clone().upcast::<gtk::Widget>(),
        fps_limit_widget.clone().upcast::<gtk::Widget>(),
        unlock_focus_widget.clone().upcast::<gtk::Widget>(),
    ];

    let update_controls = {
        let btn_sync = toggle_button.clone();
        let status_line_sync = status_line.clone();
        let status_revealer_sync = status_revealer.clone();
        let title_hbox_sync = title_hbox.clone();
        let runtime_error_sync = runtime_error.clone();
        let diagnostics_sync = diagnostics_button.clone();
        let stack_sync = stack.clone();
        let controls = controls.clone();
        let state_sync = state.clone();
        move || {
            let (is_running, action_active, runtime_status, error_message) = {
                let s = state_sync.lock().unwrap();
                (
                    s.running,
                    s.action_active,
                    s.runtime_status,
                    s.error_message.clone(),
                )
            };

            let diagnostics_visible = stack_sync
                .visible_child_name()
                .is_some_and(|name| name == "compat");
            let (button_label, button_class) = if diagnostics_visible {
                ("Back to Settings", "back-button")
            } else if is_running {
                ("Stop Anti-AFK", "stop-button")
            } else {
                ("Start Anti-AFK", "start-button")
            };
            if !btn_sync.label().is_some_and(|label| label == button_label) {
                btn_sync.set_label(button_label);
                for class_name in ["start-button", "stop-button", "back-button"] {
                    btn_sync.remove_css_class(class_name);
                }
                btn_sync.add_css_class(button_class);
            }

            let (status_label, status_class, status_visible) = if diagnostics_visible {
                ("", "", false)
            } else {
                match runtime_status {
                    RuntimeStatus::Stopped => ("Anti-AFK is off", "", false),
                    RuntimeStatus::WaitingForSober => ("Waiting for Sober", "", true),
                    RuntimeStatus::Ready => ("Anti-AFK is on", "active", false),
                    RuntimeStatus::Paused => ("Paused - mouse activity", "paused", true),
                    RuntimeStatus::PerformingAction => ("Performing action", "active", true),
                    RuntimeStatus::Error => ("Setup required", "error", true),
                }
            };
            if status_line_sync.label() != status_label {
                status_line_sync.set_label(status_label);
            }
            for class_name in ["active", "paused", "error"] {
                status_line_sync.remove_css_class(class_name);
            }
            if !status_class.is_empty() {
                status_line_sync.add_css_class(status_class);
            }
            status_revealer_sync.set_reveal_child(status_visible);
            title_hbox_sync.set_opacity(if status_visible { 0.2 } else { 1.0 });

            if !diagnostics_visible && runtime_status == RuntimeStatus::Error {
                runtime_error_sync.set_text(error_message.as_deref().unwrap_or("Unknown error"));
                runtime_error_sync.set_visible(true);
                diagnostics_sync.set_visible(true);
            } else {
                runtime_error_sync.set_visible(false);
                diagnostics_sync.set_visible(false);
            }

            for control in &controls {
                control.set_sensitive(!is_running && !action_active);
            }
            glib::ControlFlow::Continue
        }
    };

    let uc_manual = update_controls.clone();
    let state_manual = state.clone();
    let stack_manual = stack.clone();
    toggle_button.connect_clicked(move |_| {
        if stack_manual
            .visible_child_name()
            .is_some_and(|name| name == "compat")
        {
            stack_manual.set_visible_child_name("main");
            let settings = { state_manual.lock().unwrap().clone() };
            match crate::backend::preflight(&settings) {
                Ok(()) => {
                    let mut state = state_manual.lock().unwrap();
                    if state.runtime_status == RuntimeStatus::Error {
                        state.runtime_status = RuntimeStatus::Stopped;
                        state.error_message = None;
                    }
                }
                Err(error) => set_runtime_error(&state_manual, error),
            }
            uc_manual();
            return;
        }

        let is_running = { state_manual.lock().unwrap().running };
        if is_running {
            {
                let mut state = state_manual.lock().unwrap();
                state.running = false;
                state.manually_stopped = true;
            }
            set_runtime_status(&state_manual, RuntimeStatus::Stopped);
        } else {
            let settings = { state_manual.lock().unwrap().clone() };
            match crate::backend::preflight(&settings) {
                Ok(()) => {
                    {
                        let mut state = state_manual.lock().unwrap();
                        state.running = true;
                        state.manually_stopped = false;
                    }
                    set_runtime_status(&state_manual, RuntimeStatus::WaitingForSober);
                }
                Err(error) => set_runtime_error(&state_manual, error),
            }
        }
        uc_manual();
    });

    {
        let settings = { state.lock().unwrap().clone() };
        if let Err(error) = crate::backend::preflight(&settings) {
            set_runtime_error(&state, error);
        }
    }
    update_controls();
    glib::timeout_add_local(std::time::Duration::from_millis(500), update_controls);

    window.present();
    window
}

fn build_compat_ui(
    container: Box,
    stack: Stack,
    state: SharedState,
    version_widgets: VersionCheckWidgets,
) {
    container.append(
        &Label::builder()
            .label("Diagnostics")
            .css_classes(["compact-title"])
            .margin_top(8)
            .margin_bottom(10)
            .halign(Align::Center)
            .build(),
    );
    let list = ListBox::new();
    list.add_css_class("settings-list");
    list.add_css_class("diagnostics-list");
    container.append(&list);

    let version_control = Box::new(Orientation::Horizontal, 6);
    version_control.append(&version_widgets.label);
    version_control.append(&version_widgets.button);
    list.append(&create_settings_row(
        "info",
        "Version",
        None,
        &version_control,
        None,
        false,
    ));

    let (environment_name, environment_ok) = match state.lock().unwrap().mode {
        0 => ("Hyprland", true),
        1 => ("KDE Plasma 6", true),
        _ => ("Unsupported", false),
    };
    let environment_label = Label::builder()
        .label(environment_name)
        .css_classes([if environment_ok {
            "diagnostic-ok"
        } else {
            "diagnostic-error"
        }])
        .build();
    list.append(&create_settings_row(
        "settings",
        "Environment",
        None,
        &environment_label,
        None,
        false,
    ));

    let is_hyprland = crate::backend::is_hyprland();
    let is_kde = crate::backend::is_kde();
    let auto_reconnect = { state.lock().unwrap().auto_reconnect };
    let settings = { state.lock().unwrap().clone() };

    match crate::backend::preflight(&settings) {
        Ok(()) => list.append(&add_compat_item(
            "Automation Preflight",
            "",
            None,
            ItemStatus::Ok,
        )),
        Err(error) => list.append(&add_compat_item(
            "Automation Preflight",
            &error,
            None,
            ItemStatus::Error,
        )),
    }

    if is_hyprland || is_kde {
        let uinput_ok = check_uinput_permission();
        let rule_exists =
            std::path::Path::new("/etc/udev/rules.d/99-uinput-antiafk.rules").exists();

        let fix_action = if !rule_exists {
            let mini_fix = Button::builder()
                .label("Fix")
                .css_classes(["version-btn"])
                .valign(Align::Center)
                .build();

            let stack_clone = stack.clone();
            let state_clone = state.clone();
            let container_clone = container.clone();
            let version_widgets_clone = version_widgets.clone();
            mini_fix.connect_clicked(move |_| {
                let s_c = stack_clone.clone();
                let st_c = state_clone.clone();
                let c_c = container_clone.clone();

                let cmd = "echo 'KERNEL==\"uinput\", MODE=\"0666\"' > /etc/udev/rules.d/99-uinput-antiafk.rules && udevadm control --reload-rules && udevadm trigger";
                let proc = gio::Subprocess::newv(
                    &["pkexec".as_ref(), "sh".as_ref(), "-c".as_ref(), cmd.as_ref()],
                    gio::SubprocessFlags::NONE
                );

                if let Ok(p) = proc {
                    let version_widgets_for_refresh = version_widgets_clone.clone();
                    glib::spawn_future_local(async move {
                        let _ = p.wait_future().await;
                        glib::timeout_future(std::time::Duration::from_millis(500)).await;

                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(
                            c_c.clone(),
                            s_c.clone(),
                            st_c.clone(),
                            version_widgets_for_refresh,
                        );
                    });
                }
            });
            Some(mini_fix.upcast::<gtk::Widget>())
        } else {
            let remove_fix = Button::builder()
                .label("Remove Auto-Fix Rule")
                .css_classes(["version-btn"])
                .valign(Align::Center)
                .build();

            let stack_clone = stack.clone();
            let state_clone = state.clone();
            let container_clone = container.clone();
            let version_widgets_clone = version_widgets.clone();
            remove_fix.connect_clicked(move |_| {
                let s_c = stack_clone.clone();
                let st_c = state_clone.clone();
                let c_c = container_clone.clone();

                let cmd = "rm -f /etc/udev/rules.d/99-uinput-antiafk.rules && udevadm control --reload-rules && udevadm trigger";
                let proc = gio::Subprocess::newv(
                    &["pkexec".as_ref(), "sh".as_ref(), "-c".as_ref(), cmd.as_ref()],
                    gio::SubprocessFlags::NONE
                );

                if let Ok(p) = proc {
                    let version_widgets_for_refresh = version_widgets_clone.clone();
                    glib::spawn_future_local(async move {
                        let _ = p.wait_future().await;
                        glib::timeout_future(std::time::Duration::from_millis(500)).await;

                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(
                            c_c.clone(),
                            s_c.clone(),
                            st_c.clone(),
                            version_widgets_for_refresh,
                        );
                    });
                }
            });
            Some(remove_fix.upcast::<gtk::Widget>())
        };

        let status = if uinput_ok {
            ItemStatus::Ok
        } else {
            ItemStatus::Error
        };
        list.append(&add_compat_item(
            "uinput Permissions",
            "",
            fix_action,
            status,
        ));

        if is_hyprland {
            let hyprctl_ok = command_succeeds("hyprctl", &["version"]);
            let status = if hyprctl_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item("hyprctl Utility", "", None, status));

            if auto_reconnect {
                let grim_ok = command_succeeds("grim", &["-h"]);
                let status = if grim_ok {
                    ItemStatus::Ok
                } else {
                    ItemStatus::Error
                };
                list.append(&add_compat_item("grim Tool", "", None, status));
            }
        }

        if is_kde {
            let qdbus_ok = command_succeeds("qdbus6", &["--version"])
                || command_succeeds("qdbus", &["--version"]);
            let status = if qdbus_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item("qdbus Utility", "", None, status));

            let journalctl_ok = command_succeeds("journalctl", &["--version"]);
            let status = if journalctl_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item("journalctl Utility", "", None, status));

            if auto_reconnect {
                let spectacle_ok = command_succeeds("spectacle", &["--version"]);
                let status = if spectacle_ok {
                    ItemStatus::Ok
                } else {
                    ItemStatus::Error
                };
                list.append(&add_compat_item("spectacle Tool", "", None, status));
            }
        }
    }
}

type VersionCheckResult = Result<(bool, String), String>;

#[derive(Clone)]
struct VersionCheckWidgets {
    label: Label,
    button: Button,
}

fn apply_version_check_result(result: VersionCheckResult, label: &Label, button: &Button) {
    for class_name in ["diagnostic-ok", "diagnostic-warning", "diagnostic-error"] {
        label.remove_css_class(class_name);
    }

    match result {
        Ok((true, _)) => {
            label.set_text("Latest");
            label.add_css_class("diagnostic-ok");
            button.set_label("Check");
        }
        Ok((false, latest)) => {
            label.set_text(&format!("v{latest} available"));
            label.add_css_class("diagnostic-warning");
            button.set_label("Check");
        }
        Err(_) => {
            label.set_text("Unavailable");
            label.add_css_class("diagnostic-error");
            button.set_label("Retry");
        }
    }
    button.set_sensitive(true);
}

fn request_version_check(tx: &glib::Sender<VersionCheckResult>, label: &Label, button: &Button) {
    label.set_text("Checking...");
    label.remove_css_class("diagnostic-ok");
    label.remove_css_class("diagnostic-error");
    label.add_css_class("diagnostic-warning");
    button.set_sensitive(false);

    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(check_latest_version());
    });
}

fn create_version_check_widgets() -> VersionCheckWidgets {
    let label = Label::builder()
        .label(format!("v{CURRENT_VERSION}"))
        .css_classes(["diagnostic-ok"])
        .build();
    let button = Button::builder()
        .label("Check")
        .css_classes(["version-btn"])
        .build();

    #[allow(deprecated)]
    let (tx, rx) = glib::MainContext::channel::<VersionCheckResult>(glib::Priority::DEFAULT);
    let label_update = label.clone();
    let button_update = button.clone();
    rx.attach(None, move |result| {
        apply_version_check_result(result, &label_update, &button_update);
        glib::ControlFlow::Continue
    });

    let tx_click = tx.clone();
    let label_click = label.clone();
    let button_click = button.clone();
    button.connect_clicked(move |_| {
        request_version_check(&tx_click, &label_click, &button_click);
    });

    request_version_check(&tx, &label, &button);

    VersionCheckWidgets { label, button }
}

fn check_latest_version() -> Result<(bool, String), String> {
    let remote_v = Command::new("curl")
        .args([
            "-s",
            "--connect-timeout",
            "3",
            "https://raw.githubusercontent.com/agzes/AntiAFK-RBX-Sober/main/version",
        ])
        .output();

    if let Ok(output) = remote_v
        && output.status.success()
    {
        let latest_v_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !latest_v_str.is_empty() {
            return Ok((CURRENT_VERSION == latest_v_str, latest_v_str));
        }
    }

    Err("Failed to check for updates".to_string())
}

fn add_compat_item(
    name: &str,
    tutorial: &str,
    widget: Option<gtk::Widget>,
    status: ItemStatus,
) -> ListBoxRow {
    let (icon_name, status_text, status_class) = match status {
        ItemStatus::Ok => ("check", "Ready", "diagnostic-ok"),
        ItemStatus::Error => ("warning", "Error", "diagnostic-error"),
    };

    let control = Box::new(Orientation::Horizontal, 6);
    control.append(
        &Label::builder()
            .label(status_text)
            .css_classes([status_class])
            .build(),
    );
    if let Some(widget) = widget {
        control.append(&widget);
    }

    let subtitle = if tutorial.is_empty() {
        None
    } else {
        Some(tutorial)
    };
    create_settings_row(icon_name, name, subtitle, &control, None, false)
}

#[derive(Clone, Copy)]
enum ItemStatus {
    Ok,
    Error,
}
