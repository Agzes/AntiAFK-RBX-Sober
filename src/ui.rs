use crate::state::SharedState;
use gtk::gdk_pixbuf::PixbufLoader;
use gtk::gio;
use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Application, ApplicationWindow, Box, Button, CssProvider, DropDown, Image,
    Label, ListBox, ListBoxRow, Orientation, SpinButton, Stack, StringList, Switch,
};
use std::process::Command;

const CURRENT_VERSION: &str = "0.2.0";

const CSS: &str = "
    .main-window { background-color: @theme_bg_color; color: @theme_fg_color; }
    .main-box { padding: 20px; padding-bottom: 14px; }
    .header-box { margin-bottom: 20px; }
    .app-title { font-size: 24px; font-weight: 800; letter-spacing: -0.5px; }
    .version-btn { font-size: 9px; font-weight: bold; padding: 1px 5px; border-radius: 6px; background-color: alpha(@theme_fg_color, 0.08); color: @theme_fg_color; border: 1px solid alpha(@theme_fg_color, 0.12); margin-left: 8px; min-height: 18px; }
    .version-btn:hover { background-color: alpha(@theme_fg_color, 0.15); border: 1px solid alpha(@theme_fg_color, 0.2); }
    .icon-btn { padding: 0; min-width: 22px; min-height: 22px; border-radius: 6px; }
    .app-subtitle { font-size: 12px; margin-top: -2px; }
    .app-subtitle a, .badge-link a { color: inherit; text-decoration: none; font-weight: bold; }
    .app-subtitle a:hover { opacity: 1.0; text-decoration: underline; }
    .section-title { font-size: 10px; font-weight: bold; text-transform: uppercase; letter-spacing: 0.5px; opacity: 0.5; }
    .card { background-color: alpha(@theme_fg_color, 0.04); border: 1px solid alpha(@theme_fg_color, 0.08); border-radius: 12px; }
    list { background-color: transparent; border-radius: 12px; }
    row { padding: 8px 14px; border-bottom: 1px solid alpha(@theme_fg_color, 0.05); }
    row:first-child { border-top-left-radius: 12px; border-top-right-radius: 12px; }
    row:last-child { border-bottom-left-radius: 12px; border-bottom-right-radius: 12px; border-bottom: none; }
    row label.row-title { font-weight: 500; font-size: 14px; }
    row label.row-subtitle { font-size: 11px; opacity: 0.5; }
    row.sub-row > box { margin-left: 20px; opacity: 0.85; }
    row.sub-row label.row-title { font-size: 13px; }
    .info-icon { opacity: 0.4; }
    .info-icon:hover { opacity: 0.9; }
    .badge-link { padding: 2px 6px; border-radius: 6px; font-size: 9px; font-weight: 800; background-color: alpha(@theme_fg_color, 0.08); color: alpha(@theme_fg_color, 0.8); border: 1px solid alpha(@theme_fg_color, 0.12); }
    .badge-link:hover { background-color: alpha(@theme_fg_color, 0.15); border: 1px solid alpha(@theme_fg_color, 0.2); }
    .beta-badge { font-size: 9px; font-weight: 800; padding: 1px 5px; border-radius: 5px; background-color: #f5c71a; color: #000; margin-left: 0px; }
    dropdown button { padding: 0 6px; min-height: 26px; font-size: 12px; border-radius: 6px; }
    .clean-dropdown, dropdown.clean-dropdown { background-color: transparent; border: none; box-shadow: none; padding: 0; margin: 0; outline: none; }
    .clean-dropdown button, dropdown.clean-dropdown button.combo { background-color: alpha(@theme_fg_color, 0.08); border: 1px solid alpha(@theme_fg_color, 0.12); border-radius: 8px; padding: 0 10px; min-height: 24px; color: @theme_fg_color; margin: 0; box-shadow: none; outline: none; }
    .clean-dropdown button *, dropdown.clean-dropdown button.combo * { background-color: transparent; border: none; box-shadow: none; }
    .clean-dropdown button label, dropdown.clean-dropdown button.combo label { margin: 0; padding: 0; }
    .clean-dropdown button image, dropdown.clean-dropdown button.combo image { margin-left: 14px; }
    .clean-dropdown button > box, dropdown.clean-dropdown button.combo > box { spacing: 14px; column-gap: 14px; }
    .clean-dropdown button:hover, dropdown.clean-dropdown button.combo:hover { background-color: alpha(@theme_fg_color, 0.12); border: 1px solid alpha(@theme_fg_color, 0.2); }
    popover contents { background-color: @theme_bg_color; border: 1px solid alpha(@theme_fg_color, 0.15); border-radius: 20px; padding: 6px; color: @theme_fg_color; box-shadow: 0 4px 12px rgba(0,0,0,0.3); }
    popover listview, popover list { background-color: transparent; }
    popover listitem, popover row { padding: 8px 12px; border-radius: 10px; margin: 2px; transition: all 150ms ease; }
    popover listitem:hover, popover row:hover { background-color: alpha(@theme_fg_color, 0.08); }
    popover listitem:selected, popover row:selected { background-color: alpha(@theme_selected_bg_color, 0.35); color: @theme_fg_color; }
    spinbutton { min-height: 26px; font-size: 12px; border-radius: 6px; padding: 0; background-color: alpha(@theme_fg_color, 0.05); border: 1px solid alpha(@theme_fg_color, 0.1); }
    spinbutton button { background: none; border: none; padding: 0 4px; min-height: 22px; box-shadow: none; }
    spinbutton button:hover { background-color: alpha(@theme_fg_color, 0.05); }
    switch { margin: 0; transform: scale(0.85); outline: none; }
    .start-button, .stop-button { border-radius: 12px; padding: 12px; font-weight: 800; font-size: 14px; border: none; transition: all 200ms ease; margin-bottom: 10px; color: white; text-shadow: 0 1px 2px rgba(0,0,0,0.2); }
    .start-button { background-image: linear-gradient(to bottom, #2ecc71, #27ae60); box-shadow: 0 4px 0px #1e8449, 0 8px 15px -3px rgba(0,0,0,0.2); }
    .stop-button { background-image: linear-gradient(to bottom, #e74c3c, #c0392b); box-shadow: 0 4px 0px #922b21, 0 8px 15px -3px rgba(0,0,0,0.2); }
    .start-button:hover, .stop-button:hover { transform: translateY(-2px); }
    .start-button:hover { background-image: linear-gradient(to bottom, #34e07e, #2ecc71); box-shadow: 0 6px 0px #1e8449, 0 12px 20px -3px rgba(0,0,0,0.25); }
    .stop-button:hover { background-image: linear-gradient(to bottom, #ff5e4d, #e74c3c); box-shadow: 0 6px 0px #922b21, 0 12px 20px -3px rgba(0,0,0,0.25); }
    .start-button:active, .stop-button:active { transform: translateY(2px); }
    .start-button:active { box-shadow: 0 2px 0px #1e8449; }
    .stop-button:active { box-shadow: 0 2px 0px #922b21; }
    .status-badge { padding: 2px 6px; border-radius: 6px; font-size: 9px; font-weight: 800; }
    .status-badge.active { background-color: rgba(46, 204, 113, 0.25); color: #2ecc71; border: 1px solid rgba(46, 204, 113, 0.4); }
    .status-badge.inactive { background-color: alpha(@theme_fg_color, 0.08); color: @theme_fg_color; border: 1px solid alpha(@theme_fg_color, 0.12); }
    .compat-box { padding: 0px; }
    .compat-item { padding: 12px; border-radius: 12px; background: alpha(@theme_fg_color, 0.03); border: 1px solid alpha(@theme_fg_color, 0.06); margin-bottom: 8px; }
    .compat-item.error { border-left: 4px solid @error_color; background: alpha(@error_color, 0.1); }
    .compat-item.ok { border-left: 4px solid @success_color; background: alpha(@success_color, 0.1); }
    .compat-item.warning-item { border-left: 4px solid @warning_color; background: alpha(@warning_color, 0.1); }
    .compat-item.info-item { border-left: 4px solid @theme_selected_bg_color; background: alpha(@theme_selected_bg_color, 0.1); }
    .tutorial-text { font-size: 11px; opacity: 0.6; margin-top: 4px; }
    .compat-title { font-size: 16px; font-weight: 800; margin-bottom: 16px; opacity: 0.9; }
    .compat-name { font-size: 13px; font-weight: bold; }
    .welcome-title { font-size: 32px; font-weight: 800; letter-spacing: -1px; }
    .welcome-subtitle { font-size: 16px; margin-bottom: 20px; }
    .hypr-badge { background-color: alpha(@error_color, 0.2); color: @error_color; padding: 6px 14px; border-radius: 99px; font-size: 11px; font-weight: 800; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 12px; }
    .info-note { background-color: alpha(@theme_fg_color, 0.03); border: 1px solid alpha(@theme_fg_color, 0.07); padding: 20px; border-radius: 16px; margin: 10px 0; }
    .rbx-text { color: #E2231A; }
    .sober-text { color: #8fde58; }
    .error-text { color: @error_color; }
    .success-text { color: @success_color; }
    .warning-text { color: @warning_color; }
    .small-text { font-size: 11px; }

";

fn get_safe_icon(names: &[&str]) -> Image {
    for name in names {
        if gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap()).has_icon(name) {
            return Image::from_icon_name(name);
        }
    }
    Image::from_icon_name("image-missing")
}

fn check_uinput_permission() -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok()
}

fn create_row(
    title: &str,
    subtitle: Option<&str>,
    widget: &impl IsA<gtk::Widget>,
    info_text: Option<&str>,
    is_beta: bool,
    is_sub: bool,
) -> (ListBoxRow, gtk::Widget) {
    let row = ListBoxRow::new();
    let main_hbox = Box::new(Orientation::Horizontal, 12);
    main_hbox.set_valign(Align::Center);
    if is_sub {
        row.add_css_class("sub-row");
    }
    let text_vbox = Box::new(Orientation::Vertical, 0);
    text_vbox.set_valign(Align::Center);
    let title_hbox = Box::new(Orientation::Horizontal, 4);
    title_hbox.set_valign(Align::Center);
    if let Some(txt) = info_text {
        let info_img = get_safe_icon(&[
            "info-symbolic",
            "help-info-symbolic",
            "dialog-information-symbolic",
        ]);
        info_img.add_css_class("info-icon");
        info_img.set_tooltip_text(Some(txt));
        info_img.set_property("name", "row-info-icon");
        title_hbox.append(&info_img);
    }
    let display_title = if is_sub {
        format!("↳ {title}")
    } else {
        title.to_string()
    };
    title_hbox.append(
        &Label::builder()
            .label(&display_title)
            .halign(Align::Start)
            .css_classes(["row-title"])
            .build(),
    );
    if is_beta {
        let beta_lbl = Label::builder()
            .label("BETA")
            .css_classes(["beta-badge"])
            .valign(Align::Center)
            .build();
        title_hbox.append(&beta_lbl);
    }
    text_vbox.append(&title_hbox);
    if let Some(sub) = subtitle {
        let sub_label = Label::builder()
            .label(sub)
            .halign(Align::Start)
            .css_classes(["row-subtitle"])
            .build();
        text_vbox.append(&sub_label);
    }
    main_hbox.append(&text_vbox);
    let filler = Box::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    main_hbox.append(&filler);
    widget.set_valign(Align::Center);
    main_hbox.append(widget);
    row.set_child(Some(&main_hbox));
    row.set_activatable(false);
    row.set_selectable(false);
    (row, widget.clone().upcast::<gtk::Widget>())
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
        .default_width(420)
        .default_height(580)
        .resizable(false)
        .build();
    window.add_css_class("main-window");
    window.set_icon_name(Some(crate::state::APP_ID));

    let root_vbox = Box::new(Orientation::Vertical, 0);
    root_vbox.add_css_class("main-box");
    window.set_child(Some(&root_vbox));

    let load_pb = |data: &[u8]| {
        let loader = PixbufLoader::new();
        loader.write(data).ok();
        loader.close().ok();
        loader.pixbuf()
    };

    let pb_logo = load_pb(include_bytes!("../assets/logo.png"));
    let pb_off = load_pb(include_bytes!("../assets/tray-off.png"));
    let pb_run = load_pb(include_bytes!("../assets/tray-run.png"));

    let header_box = Box::new(Orientation::Horizontal, 12);
    header_box.add_css_class("header-box");

    let icon_stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(400)
        .build();

    let img_logo = Image::builder().pixel_size(44).build();
    if let Some(pb) = pb_logo {
        img_logo.set_from_pixbuf(Some(&pb));
    }
    let img_off = Image::builder().pixel_size(44).build();
    if let Some(pb) = pb_off {
        img_off.set_from_pixbuf(Some(&pb));
    }
    let img_run = Image::builder().pixel_size(44).build();
    if let Some(pb) = pb_run {
        img_run.set_from_pixbuf(Some(&pb));
    }

    icon_stack.add_named(&img_logo, Some("logo"));
    icon_stack.add_named(&img_off, Some("off"));
    icon_stack.add_named(&img_run, Some("run"));

    let click_gesture = gtk::GestureClick::new();
    let stack_cycle = icon_stack.clone();
    click_gesture.connect_pressed(move |_, _, _, _| {
        let current = stack_cycle
            .visible_child_name()
            .map(|s| s.to_string())
            .unwrap_or_default();
        match current.as_str() {
            "logo" => stack_cycle.set_visible_child_name("off"),
            "off" => stack_cycle.set_visible_child_name("run"),
            _ => stack_cycle.set_visible_child_name("logo"),
        }
    });
    icon_stack.add_controller(click_gesture);
    icon_stack.set_cursor_from_name(Some("pointer"));

    header_box.append(&icon_stack);
    let title_vbox = Box::new(Orientation::Vertical, 0);
    let title_hbox = Box::new(Orientation::Horizontal, 0);
    title_hbox.set_valign(Align::Center);
    title_hbox.append(
        &Label::builder()
            .label("AntiAFK-")
            .css_classes(["app-title"])
            .build(),
    );
    let rbx_label = Label::builder()
        .label("RBX")
        .css_classes(["app-title", "rbx-text"])
        .build();
    title_hbox.append(&rbx_label);

    let combined_btn = Button::builder()
        .css_classes(["version-btn"])
        .valign(Align::Center)
        .has_frame(false)
        .build();

    let btn_content = Box::new(Orientation::Horizontal, 4);
    btn_content.append(
        &Label::builder()
            .label(format!("v{CURRENT_VERSION}"))
            .build(),
    );
    let settings_icon = get_safe_icon(&[
        "preferences-system-symbolic",
        "emblem-system-symbolic",
        "settings-symbolic",
    ]);
    settings_icon.set_pixel_size(15);
    btn_content.append(&settings_icon);
    combined_btn.set_child(Some(&btn_content));

    title_hbox.append(&combined_btn);

    title_vbox.append(&title_hbox);
    let sober_label = Label::builder()
        .use_markup(true)
        .label("<b>Sober Edition</b>")
        .halign(Align::Start)
        .css_classes(["app-subtitle", "sober-text"])
        .build();
    let by_label = Label::builder()
        .use_markup(true)
        .label(" • by <a href='https://github.com/agzes'>agzes</a>")
        .halign(Align::Start)
        .css_classes(["app-subtitle"])
        .build();
    let sub_hbox = Box::new(Orientation::Horizontal, 0);
    sub_hbox.append(&sober_label);
    sub_hbox.append(&by_label);
    title_vbox.append(&sub_hbox);
    header_box.append(&title_vbox);
    let filler = Box::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    header_box.append(&filler);

    let right_vbox = Box::new(Orientation::Vertical, 4);
    right_vbox.set_valign(Align::Center);
    right_vbox.set_halign(Align::End);

    let stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .build();

    let top_hbox = Box::new(Orientation::Horizontal, 8);
    top_hbox.set_halign(Align::End);

    let status_badge = Label::builder()
        .label("IDLE")
        .css_classes(["status-badge", "inactive"])
        .halign(Align::End)
        .build();
    top_hbox.append(&status_badge);
    right_vbox.append(&top_hbox);
    right_vbox.append(
        &Label::builder()
            .use_markup(true)
            .label("<a href='https://github.com/agzes/AntiAFK-RBX-Sober'>GITHUB</a>")
            .css_classes(["badge-link"])
            .halign(Align::End)
            .build(),
    );
    header_box.append(&right_vbox);
    root_vbox.append(&header_box);

    let main_vbox = Box::new(Orientation::Vertical, 0);
    main_vbox.append(&Box::builder().height_request(8).build());
    let compat_vbox = Box::new(Orientation::Vertical, 0);
    compat_vbox.set_vexpand(true);

    let warning_vbox = Box::new(Orientation::Vertical, 0);
    warning_vbox.set_vexpand(true);

    warning_vbox.append(&Box::builder().height_request(80).build());

    let welcome_icon = get_safe_icon(&[
        "dialog-warning-symbolic",
        "emblem-important-symbolic",
        "warning-symbolic",
    ]);
    welcome_icon.set_pixel_size(80);
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
            "<span size='large' weight='800'>Wayland Only • WIP</span>\n\n\
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
    root_vbox.append(&stack);

    let compat_vbox_clone = compat_vbox.clone();
    let stack_clone = stack.clone();
    let state_clone = state.clone();
    let refresh_compat = move || {
        while let Some(child) = compat_vbox_clone.first_child() {
            compat_vbox_clone.remove(&child);
        }
        build_compat_ui(
            compat_vbox_clone.clone(),
            stack_clone.clone(),
            state_clone.clone(),
        );
        stack_clone.set_visible_child_name("compat");
    };

    let rc = refresh_compat.clone();
    combined_btn.connect_clicked(move |_| {
        rc();
    });

    let (last_version, shown_warning) = {
        let s = state.lock().unwrap();
        (s.last_run_version.clone(), s.shown_warning)
    };

    if !shown_warning {
        stack.set_visible_child_name("warning");
    } else if last_version.is_none_or(|v| v != CURRENT_VERSION) {
        refresh_compat();
    } else {
        stack.set_visible_child_name("main");
    }

    let btn_container = Box::builder().orientation(Orientation::Vertical).build();
    main_vbox.append(&btn_container);

    let toggle_button = Button::builder().label("Start Anti-AFK").build();
    toggle_button.add_css_class("start-button");
    btn_container.append(&toggle_button);

    let mode_warning = Label::builder()
        .label("⚠ Selected method is not supported in your desktop.")
        .visible(false)
        .margin_bottom(6)
        .css_classes(["error-text", "small-text"])
        .build();
    btn_container.append(&mode_warning);

    let perm_warning = Label::builder()
        .label("⚠ Permission Denied: Run sudo chmod 666 /dev/uinput")
        .halign(Align::Center)
        .wrap(true)
        .margin_bottom(12)
        .visible(!check_uinput_permission())
        .css_classes(["error-text", "small-text"])
        .build();
    btn_container.append(&perm_warning);

    main_vbox.append(
        &Label::builder()
            .label("Input & Action")
            .css_classes(["section-title"])
            .margin_start(4)
            .margin_top(10)
            .build(),
    );
    let core_list = ListBox::new();
    core_list.add_css_class("card");
    let initial_state = { state.lock().unwrap().clone() };
    let mode_names = vec!["Swapper", "Plasma (preview)", "Other Desktops"];
    let is_hypr_detected = crate::backend::is_hyprland();
    let is_plasma_detected = crate::backend::is_plasma();

    let list_factory = gtk::SignalListItemFactory::new();
    list_factory.connect_setup(move |_, list_item| {
        let box_ = Box::new(Orientation::Vertical, 0);
        let title = Label::builder()
            .halign(Align::Start)
            .use_markup(true)
            .build();
        let subtitle = Label::builder()
            .halign(Align::Start)
            .css_classes(["row-subtitle"])
            .build();
        let wip = Label::builder()
            .halign(Align::Start)
            .use_markup(true)
            .build();
        subtitle.set_margin_top(-2);
        box_.append(&title);
        box_.append(&subtitle);
        box_.append(&wip);
        list_item
            .downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&box_));
    });

    list_factory.connect_bind(move |_, list_item| {
        let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
        let box_ = item.child().unwrap().downcast::<Box>().unwrap();
        let title = box_.first_child().unwrap().downcast::<Label>().unwrap();
        let subtitle = title.next_sibling().unwrap().downcast::<Label>().unwrap();
        let wip = subtitle
            .next_sibling()
            .unwrap()
            .downcast::<Label>()
            .unwrap();

        let pos = item.position();
        if pos == 0 {
            title.set_markup("<b>Swapper</b>");
            subtitle.set_label("Desktops: Hyprland");
            if is_hypr_detected {
                wip.set_markup(
                    "<span size='smaller' color='#8fde58' weight='bold'>Recommended</span>",
                );
            } else {
                wip.set_markup(
                    "<span size='smaller' color='#ff5555' weight='bold'>Not supported</span>",
                );
            }
        } else if pos == 1 {
            title.set_markup("<b>Plasma (preview)</b>");
            subtitle.set_label("Desktops: KDE Plasma 6");
            if is_plasma_detected {
                wip.set_markup(
                    "<span size='smaller' color='#8fde58' weight='bold'>Recommended</span>",
                );
            } else {
                wip.set_markup(
                    "<span size='smaller' color='#ff5555' weight='bold'>Not supported</span>",
                );
            }
        } else {
            title.set_markup("<b>Other Environments</b>");
            subtitle.set_label("GNOME, X11");
            wip.set_markup(
                "<span size='smaller' color='#ff5555' weight='bold'>WIP / Planned</span>",
            );
        }
    });

    let selected_factory = gtk::SignalListItemFactory::new();
    selected_factory.connect_setup(move |_, list_item| {
        let label = Label::builder().halign(Align::Start).build();
        list_item
            .downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&label));
    });
    selected_factory.connect_bind(move |_, list_item| {
        let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
        let label = item.child().unwrap().downcast::<Label>().unwrap();
        let pos = item.position();
        if pos == 0 {
            label.set_label("Swapper");
        } else if pos == 1 {
            label.set_label("Plasma (preview)");
        } else {
            label.set_label("Other (WIP)");
        }
    });

    let mode_dropdown = DropDown::builder()
        .model(&StringList::new(&mode_names))
        .factory(&selected_factory)
        .list_factory(&list_factory)
        .selected(initial_state.mode as u32)
        .build();
    mode_dropdown.add_css_class("clean-dropdown");

    let mode_info =
        "Methods of performing actions and simulating user activity in the game windows.";
    let (row, _) = create_row(
        "Input Method",
        Some("Simulation & Action methods"),
        &mode_dropdown,
        Some(mode_info),
        false,
        false,
    );
    core_list.append(&row);
    let action_idx = if initial_state.jump {
        0
    } else if initial_state.walk {
        1
    } else {
        2
    };
    let action_dropdown = DropDown::builder()
        .model(&StringList::new(&[
            "Jump (Space)",
            "Walk (W/S)",
            "Zoom (I/O)",
        ]))
        .selected(action_idx)
        .build();
    action_dropdown.add_css_class("clean-dropdown");
    let (row, _) = create_row(
        "AFK Action",
        Some("Select character action"),
        &action_dropdown,
        None,
        false,
        false,
    );
    core_list.append(&row);
    let adj = Adjustment::new(
        initial_state.interval_seq as f64,
        1.0,
        1200.0,
        1.0,
        10.0,
        0.0,
    );
    let interval_spin = SpinButton::builder()
        .adjustment(&adj)
        .climb_rate(1.0)
        .digits(0)
        .numeric(true)
        .build();
    let (row, _) = create_row(
        "AFK Interval (s)",
        Some("Time between AFK cycles"),
        &interval_spin,
        None,
        false,
        false,
    );
    core_list.append(&row);
    main_vbox.append(&core_list);

    main_vbox.append(
        &Label::builder()
            .label("Automation & Other")
            .css_classes(["section-title"])
            .margin_start(4)
            .margin_top(14)
            .build(),
    );
    let auto_list = ListBox::new();
    auto_list.add_css_class("card");
    let auto_start_sw = Switch::new();
    auto_start_sw.set_active(initial_state.auto_start);
    let (row, _) = create_row(
        "Auto-Start",
        Some("Enable AFK when Sober is detected"),
        &auto_start_sw,
        None,
        false,
        false,
    );
    auto_list.append(&row);
    let user_safe_sw = Switch::new();
    user_safe_sw.set_active(initial_state.user_safe);
    let (user_safe_row, _) = create_row(
        "User-Safe",
        Some("Pause action on activity"),
        &user_safe_sw,
        Some("Note: Plasma only detects cursor movement"),
        true,
        false,
    );
    auto_list.append(&user_safe_row);
    let multi_instance_sw = Switch::new();
    multi_instance_sw.set_active(initial_state.multi_instance);
    let (row, _) = create_row(
        "Multi-Instance",
        Some("Support multiple game clients"),
        &multi_instance_sw,
        None,
        false,
        false,
    );
    auto_list.append(&row);

    let stealth_sw = Switch::new();
    stealth_sw.set_active(initial_state.stealth);
    let (stealth_row, _) = create_row(
        "Stealth Mode",
        Some("Minimize window after actions"),
        &stealth_sw,
        None,
        false,
        false,
    );
    auto_list.append(&stealth_row);
    let reconnect_sw = Switch::new();
    reconnect_sw.set_active(initial_state.auto_reconnect);
    let (row, _) = create_row(
        "Auto Reconnect",
        Some("Auto-click 'Reconnect' button"),
        &reconnect_sw,
        None,
        true,
        false,
    );
    auto_list.append(&row);

    let fps_capper_sw = Switch::new();
    fps_capper_sw.set_active(initial_state.fps_capper);
    let (row, _) = create_row(
        "FPS Capper",
        Some("Limit background process resources"),
        &fps_capper_sw,
        None,
        true,
        false,
    );
    auto_list.append(&row);
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
    let (fps_limit_row, fps_limit_widget) = create_row(
        "CPU Quota (%)",
        Some("Max CPU time allowed"),
        &fps_limit_spin,
        None,
        false,
        true,
    );
    fps_limit_row.set_visible(initial_state.fps_capper);
    auto_list.append(&fps_limit_row);
    let unlock_focus_sw = Switch::new();
    unlock_focus_sw.set_active(initial_state.stop_limit_on_focus);
    let (unlock_focus_row, unlock_focus_widget) = create_row(
        "Unlock at Focus",
        Some("Disable limit when window active"),
        &unlock_focus_sw,
        None,
        false,
        true,
    );
    unlock_focus_row.set_visible(initial_state.fps_capper);
    auto_list.append(&unlock_focus_row);

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
    main_vbox.append(&auto_list);

    let stealth_live = stealth_sw.clone();
    let update_state = {
        let state_arc = state.clone();
        let mode_dd_live = mode_dropdown.clone();
        let interval_spin_live = interval_spin.clone();
        let action_dd_live = action_dropdown.clone();
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
            s.mode = mode_dd_live.selected() as usize;
            let action_idx = action_dd_live.selected();
            s.jump = action_idx == 0;
            s.walk = action_idx == 1;
            s.spin_jiggle = action_idx == 2;
            s.interval_seq = interval_spin_live.value() as u64;
            if auto_start_live.is_active() && !s.auto_start {
                s.manually_stopped = false;
            }
            s.auto_start = auto_start_live.is_active();
            s.multi_instance = multi_instance_live.is_active();
            let st = stealth_live.is_active();
            s.stealth = st;
            s.hides_game = st;
            s.user_safe = user_safe_live.is_active();
            s.auto_reconnect = auto_reconnect_live.is_active();
            s.fps_capper = fps_capper_live.is_active();
            s.fps_limit = fps_limit_live.value() as u32;
            s.stop_limit_on_focus = stop_limit_on_focus_live.is_active();
            s.save();
        }
    };

    let us = update_state.clone();
    mode_dropdown.connect_selected_notify(move |_| us());
    let us = update_state.clone();
    action_dropdown.connect_selected_notify(move |_| us());
    let us = update_state.clone();
    interval_spin.connect_value_changed(move |_| us());
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
        mode_dropdown.clone().upcast::<gtk::Widget>(),
        action_dropdown.clone().upcast::<gtk::Widget>(),
        interval_spin.clone().upcast::<gtk::Widget>(),
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
        let status_badge_sync = status_badge.clone();
        let controls = controls.clone();
        let state_sync = state.clone();
        move || {
            let (is_running, action_active) = {
                let s = state_sync.lock().unwrap();
                (s.running, s.action_active)
            };
            let btn_is_stop = btn_sync.label().is_some_and(|l| l.contains("Stop"));
            if is_running != btn_is_stop {
                if is_running {
                    btn_sync.set_label("Stop Anti-AFK");
                    btn_sync.add_css_class("stop-button");
                    btn_sync.remove_css_class("start-button");
                    status_badge_sync.set_label("ACTIVE");
                    status_badge_sync.add_css_class("active");
                    status_badge_sync.remove_css_class("inactive");
                } else {
                    btn_sync.set_label("Start Anti-AFK");
                    btn_sync.add_css_class("start-button");
                    btn_sync.remove_css_class("stop-button");
                    status_badge_sync.set_label("IDLE");
                    status_badge_sync.add_css_class("inactive");
                    status_badge_sync.remove_css_class("active");
                }
            }
            for c in &controls {
                c.set_sensitive(!is_running && !action_active);
            }
            glib::ControlFlow::Continue
        }
    };

    let uc_manual = update_controls.clone();
    let state_manual = state.clone();
    toggle_button.connect_clicked(move |_| {
        let mut s = state_manual.lock().unwrap();
        s.running = !s.running;
        s.manually_stopped = !s.running;
        drop(s);
        uc_manual();
    });

    glib::timeout_add_local(std::time::Duration::from_millis(500), update_controls);

    let toggle_btn_restrict = toggle_button.clone();
    let mode_warn_restrict = mode_warning.clone();
    let perm_warn_restrict = perm_warning.clone();
    let multi_instance_restrict = multi_instance_sw.clone();
    let user_safe_restrict = user_safe_sw.clone();
    let reconnect_restrict = reconnect_sw.clone();
    let is_hyprland = crate::backend::is_hyprland();
    let is_plasma = crate::backend::is_plasma();

    let user_safe_row_live = user_safe_row.clone();
    let stealth_row_live = stealth_row.clone();
    let update_mode_ui = move |selected_idx: u32| {
        let user_safe_row = user_safe_row_live.clone();
        let stealth_row = stealth_row_live.clone();
        let is_swapper = selected_idx == 0;
        let is_plasma_mode = selected_idx == 1;
        let is_other = selected_idx == 2;
        let has_uinput = check_uinput_permission();

        perm_warn_restrict.set_visible(!has_uinput);

        let mut invalid = false;
        if is_swapper && !is_hyprland {
            invalid = true;
            mode_warn_restrict.set_markup("<span size='small' color='#ff5555'>⚠ Swapper requires Hyprland. Your desktop is not supported.</span>");
        } else if is_plasma_mode && !is_plasma {
            invalid = true;
            mode_warn_restrict.set_markup("<span size='small' color='#ff5555'>⚠ Plasma (preview) requires KDE Plasma 6. Your desktop is not supported.</span>");
        } else if is_other {
            invalid = true;
            mode_warn_restrict.set_markup("<span size='small' color='#ff5555'>⚠ This method is in development (WIP) and cannot be started.</span>");
        }

        mode_warn_restrict.set_visible(invalid);
        toggle_btn_restrict.set_sensitive(!invalid && has_uinput);

        multi_instance_restrict.set_sensitive(is_swapper || is_plasma_mode);
        user_safe_restrict.set_sensitive(is_swapper || is_plasma_mode);
        stealth_row.set_sensitive(is_swapper || is_plasma_mode);

        let mut curr = user_safe_row.first_child();
        while let Some(c1) = curr {
            let mut curr2 = c1.first_child();
            while let Some(c2) = curr2 {
                let mut curr3 = c2.first_child();
                while let Some(c3) = curr3 {
                    let mut curr4 = c3.first_child();
                    while let Some(c4) = curr4 {
                        let name = c4.widget_name();
                        if name == "row-info-icon" || name == "row-beta-badge" {
                            c4.set_visible(is_plasma_mode);
                        }
                        curr4 = c4.next_sibling();
                    }
                    curr3 = c3.next_sibling();
                }
                curr2 = c2.next_sibling();
            }
            curr = c1.next_sibling();
        }

        reconnect_restrict.set_sensitive(is_swapper || is_plasma_mode);
    };

    let umi_hover = update_mode_ui.clone();
    let mode_dd_hover = mode_dropdown.clone();
    let hover_controller = gtk::EventControllerMotion::new();
    hover_controller.connect_enter(move |_, _, _| {
        umi_hover(mode_dd_hover.selected());
    });
    btn_container.add_controller(hover_controller);

    let umi = update_mode_ui.clone();
    mode_dropdown.connect_selected_notify(move |dd: &DropDown| {
        umi(dd.selected());
    });

    update_mode_ui(mode_dropdown.selected());

    window.present();
    window
}

fn build_compat_ui(container: Box, stack: Stack, state: SharedState) {
    container.append(
        &Label::builder()
            .label("Compatibility Check")
            .css_classes(["compat-title"])
            .halign(Align::Start)
            .build(),
    );
    let list = Box::new(Orientation::Vertical, 0);
    container.append(&list);

    let version_box = Box::new(Orientation::Vertical, 0);
    list.append(&version_box);
    version_box.append(&add_compat_item(
        "Application Version",
        "Checking for updates...",
        None,
        ItemStatus::Ok,
    ));

    #[allow(deprecated)]
    let (tx, rx) =
        glib::MainContext::channel::<Result<(bool, String), String>>(glib::Priority::DEFAULT);

    let tx_thread = tx.clone();
    std::thread::spawn(move || {
        let res = check_latest_version();
        let _ = tx_thread.send(res);
    });

    let version_box_v = version_box.clone();
    let stack_v = stack.clone();
    let state_v = state.clone();
    let container_v = container.clone();

    rx.attach(None, move |result| {
        while let Some(child) = version_box_v.first_child() {
            version_box_v.remove(&child);
        }

        let render_version =
            |res: Result<(bool, String), String>, vb: Box, s: Stack, st: SharedState, c: Box| {
                while let Some(child) = vb.first_child() {
                    vb.remove(&child);
                }
                match res {
                    Ok((version_ok, latest_v)) => {
                        let version_tutorial = if version_ok {
                            format!("Current: v{CURRENT_VERSION}. You have the latest version.")
                        } else {
                            format!("Update available: v{latest_v}. Visit GitHub to download.")
                        };
                        let check_btn = Button::builder()
                            .label(if version_ok { "CHECK" } else { "UPDATE" })
                            .css_classes(["version-btn"])
                            .valign(Align::Center)
                            .build();
                        let s_c = s.clone();
                        let st_c = st.clone();
                        let c_c = c.clone();
                        let v_ok = version_ok;
                        check_btn.connect_clicked(move |_| {
                            if v_ok {
                                while let Some(child) = c_c.first_child() {
                                    c_c.remove(&child);
                                }
                                build_compat_ui(c_c.clone(), s_c.clone(), st_c.clone());
                            } else {
                                let _ = Command::new("xdg-open")
                                    .arg("https://github.com/agzes/AntiAFK-RBX-Sober/releases")
                                    .spawn();
                            }
                        });
                        let status = if version_ok {
                            ItemStatus::Ok
                        } else {
                            ItemStatus::Info
                        };
                        vb.append(&add_compat_item(
                            "Application Version",
                            &version_tutorial,
                            Some(check_btn.upcast()),
                            status,
                        ));
                    }
                    Err(e) => {
                        let retry_btn = Button::builder()
                            .label("RETRY")
                            .css_classes(["version-btn"])
                            .valign(Align::Center)
                            .build();
                        let s_c = s.clone();
                        let st_c = st.clone();
                        let c_c = c.clone();
                        retry_btn.connect_clicked(move |_| {
                            while let Some(child) = c_c.first_child() {
                                c_c.remove(&child);
                            }
                            build_compat_ui(c_c.clone(), s_c.clone(), st_c.clone());
                        });
                        vb.append(&add_compat_item(
                            "Application Version",
                            &e,
                            Some(retry_btn.upcast()),
                            ItemStatus::Warning,
                        ));
                    }
                }
            };

        render_version(
            result,
            version_box_v.clone(),
            stack_v.clone(),
            state_v.clone(),
            container_v.clone(),
        );

        glib::ControlFlow::Break
    });

    let is_hyprland = crate::backend::is_hyprland();
    let is_plasma = crate::backend::is_plasma();

    if is_hyprland || is_plasma {
        let uinput_ok = check_uinput_permission();
        let rule_exists =
            std::path::Path::new("/etc/udev/rules.d/99-uinput-antiafk.rules").exists();

        let fix_action = if !uinput_ok || !rule_exists {
            let mini_fix = Button::builder()
                .label("FIX")
                .css_classes(["version-btn"])
                .valign(Align::Center)
                .build();

            let stack_clone = stack.clone();
            let state_clone = state.clone();
            let container_clone = container.clone();
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
                    glib::spawn_future_local(async move {
                        let _ = p.wait_future().await;
                        glib::timeout_future(std::time::Duration::from_millis(500)).await;

                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(c_c.clone(), s_c.clone(), st_c.clone());
                    });
                }
            });
            Some(mini_fix.upcast::<gtk::Widget>())
        } else {
            None
        };

        let status = if uinput_ok {
            ItemStatus::Ok
        } else {
            ItemStatus::Error
        };
        list.append(&add_compat_item(
            "uinput Permissions",
            "Access to /dev/uinput required for simulation.",
            fix_action,
            status,
        ));

        if is_hyprland {
            let hyprctl_ok = Command::new("hyprctl").arg("version").output().is_ok();
            let status = if hyprctl_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item(
                "hyprctl Utility",
                "Required for window control on Hyprland.",
                None,
                status,
            ));

            let grim_ok = Command::new("grim").arg("-h").output().is_ok();
            let status = if grim_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item(
                "grim Tool",
                "Required for Auto-Reconnect (pixel scanning).",
                None,
                status,
            ));
        }

        if is_plasma {
            let qdbus_ok = Command::new("qdbus6").arg("--version").output().is_ok();
            let status = if qdbus_ok {
                ItemStatus::Ok
            } else {
                ItemStatus::Error
            };
            list.append(&add_compat_item(
                "qdbus6 Utility",
                "Required for window control on KDE Plasma 6.",
                None,
                status,
            ));
        }
    } else {
        list.append(&add_compat_item(
            "Compatibility",
            "This project currently supports only Hyprland or KDE Plasma 6 (Wayland).",
            None,
            ItemStatus::Error,
        ));
    }

    let spacer = Box::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    container.append(&spacer);

    if is_hyprland || is_plasma {
        let _uinput_ok = check_uinput_permission();
        let rule_exists =
            std::path::Path::new("/etc/udev/rules.d/99-uinput-antiafk.rules").exists();

        if !rule_exists {
            let fix_btn = Button::builder()
                .label("Auto-Fix Permissions")
                .css_classes(["version-btn"])
                .halign(Align::Center)
                .margin_bottom(10)
                .build();

            let stack_clone = stack.clone();
            let state_clone = state.clone();
            let container_clone = container.clone();
            fix_btn.connect_clicked(move |_| {
                let s_c = stack_clone.clone();
                let st_c = state_clone.clone();
                let c_c = container_clone.clone();

                let cmd = "echo 'KERNEL==\"uinput\", MODE=\"0666\"' > /etc/udev/rules.d/99-uinput-antiafk.rules && udevadm control --reload-rules && udevadm trigger";
                let proc = gio::Subprocess::newv(
                    &["pkexec".as_ref(), "sh".as_ref(), "-c".as_ref(), cmd.as_ref()],
                    gio::SubprocessFlags::NONE
                );

                if let Ok(p) = proc {
                    glib::spawn_future_local(async move {
                        let _ = p.wait_future().await;
                        glib::timeout_future(std::time::Duration::from_millis(500)).await;
                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(c_c.clone(), s_c.clone(), st_c.clone());
                    });
                }
            });
            container.append(&fix_btn);
        }

        if rule_exists {
            let remove_btn = Button::builder()
                .label("Remove Auto-Fix Rule")
                .css_classes(["version-btn"])
                .halign(Align::Center)
                .margin_bottom(10)
                .build();

            let stack_clone = stack.clone();
            let state_clone = state.clone();
            let container_clone = container.clone();
            remove_btn.connect_clicked(move |_| {
                let s_c = stack_clone.clone();
                let st_c = state_clone.clone();
                let c_c = container_clone.clone();

                let cmd = "rm -f /etc/udev/rules.d/99-uinput-antiafk.rules && udevadm control --reload-rules && udevadm trigger";
                let proc = gio::Subprocess::newv(
                    &["pkexec".as_ref(), "sh".as_ref(), "-c".as_ref(), cmd.as_ref()],
                    gio::SubprocessFlags::NONE
                );

                if let Ok(p) = proc {
                    glib::spawn_future_local(async move {
                        let _ = p.wait_future().await;
                        glib::timeout_future(std::time::Duration::from_millis(500)).await;
                        while let Some(child) = c_c.first_child() {
                            c_c.remove(&child);
                        }
                        build_compat_ui(c_c.clone(), s_c.clone(), st_c.clone());
                    });
                }
            });
            container.append(&remove_btn);
        }
    }

    let continue_btn = Button::builder()
        .label("Return to Dashboard")
        .css_classes(["start-button"])
        .build();
    let stack_clone = stack.clone();
    let state_clone = state.clone();
    continue_btn.connect_clicked(move |_| {
        let mut s = state_clone.lock().unwrap();
        s.last_run_version = Some(CURRENT_VERSION.to_string());
        s.save();
        stack_clone.set_visible_child_name("main");
    });
    container.append(&continue_btn);
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
) -> Box {
    let item = Box::new(Orientation::Vertical, 2);
    item.add_css_class("compat-item");
    let icon_name = match status {
        ItemStatus::Ok => {
            item.add_css_class("ok");
            "emblem-ok-symbolic"
        }
        ItemStatus::Error => {
            item.add_css_class("error");
            "dialog-error-symbolic"
        }
        ItemStatus::Warning => {
            item.add_css_class("warning-item");
            "dialog-warning-symbolic"
        }
        ItemStatus::Info => {
            item.add_css_class("info-item");
            "dialog-information-symbolic"
        }
    };

    let header = Box::new(Orientation::Horizontal, 10);
    let icon = match icon_name {
        "emblem-ok-symbolic" => get_safe_icon(&[
            "emblem-ok-symbolic",
            "applied-symbolic",
            "object-select-symbolic",
            "check-symbolic",
        ]),
        "dialog-error-symbolic" => get_safe_icon(&[
            "dialog-error-symbolic",
            "software-update-urgent-symbolic",
            "error-symbolic",
        ]),
        "dialog-warning-symbolic" => get_safe_icon(&[
            "dialog-warning-symbolic",
            "emblem-important-symbolic",
            "warning-symbolic",
        ]),
        "dialog-information-symbolic" => get_safe_icon(&[
            "dialog-information-symbolic",
            "info-symbolic",
            "emblem-info-symbolic",
        ]),
        "preferences-system-symbolic" => get_safe_icon(&[
            "preferences-system-symbolic",
            "emblem-system-symbolic",
            "settings-symbolic",
        ]),
        "help-browser-symbolic" => get_safe_icon(&[
            "help-browser-symbolic",
            "help-contents-symbolic",
            "help-info-symbolic",
        ]),
        _ => Image::from_icon_name(icon_name),
    };
    header.append(&icon);
    header.append(
        &Label::builder()
            .label(name)
            .css_classes(["compat-name"])
            .build(),
    );

    let filler = Box::new(Orientation::Horizontal, 0);
    filler.set_hexpand(true);
    header.append(&filler);

    if let Some(w) = widget {
        header.append(&w);
    }

    item.append(&header);
    let tut = Label::builder()
        .label(tutorial)
        .css_classes(["tutorial-text"])
        .halign(Align::Start)
        .wrap(true)
        .build();
    item.append(&tut);
    item
}

#[derive(Clone, Copy)]
enum ItemStatus {
    Ok,
    Error,
    Warning,
    Info,
}
