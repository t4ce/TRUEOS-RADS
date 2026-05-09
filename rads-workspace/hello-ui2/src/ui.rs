use v::vui2;

pub const APP_ID: &str = "dev.trueos.hello-ui2";
pub const APP_DISPLAY_NAME: &str = "Hello UI2";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = WINDOW_0_HTML;
pub const MAIN_CSS: &str = WINDOW_0_CSS;
pub const MAIN_WINDOW_DECORATIONS: &str = WINDOW_0_DECORATIONS;
pub const WINDOW_0_MODEL: &str = include_str!("../ui/main.ui2.json");
pub const WINDOW_0_HTML: &str = include_str!("../ui/index.html");
pub const WINDOW_0_CSS: &str = include_str!("../ui/styles.css");
pub const WINDOW_0_DECORATIONS: &str = "{ mode: system, titlebar: true, bottom_bar: false, title_icon: true, toggle_composition: false, fork: false, close: true, minimize: false, restore: false, maximize: false, preserve_vm: false, resizable: false, resize_button: false, rotate_buttons: false, always_on_top: false }";
pub const WINDOW_1_MODEL: &str = include_str!("../ui/windows/window2.ui2.json");
pub const WINDOW_1_HTML: &str = include_str!("../ui/windows/window2.html");
pub const WINDOW_1_CSS: &str = include_str!("../ui/windows/window2.css");
pub const WINDOW_1_DECORATIONS: &str = "{ mode: system, titlebar: true, bottom_bar: true, title_icon: true, toggle_composition: true, fork: true, close: true, minimize: true, restore: true, maximize: true, preserve_vm: true, resizable: true, resize_button: true, rotate_buttons: false, always_on_top: false }";


pub fn create_main_window() -> Option<vui2::OwnedWindow> {
    create_window_0()
}

pub fn create_window_0() -> Option<vui2::OwnedWindow> {
    let rect = vui2::Rect {
        x: 60,
        y: 60,
        width: 960,
        height: 640,
    };
    let options = vui2::CreateOptions {
        decorations: vui2::WindowDecorationOptions {
            mode: vui2::WindowDecorationMode::System,
            titlebar_visible: true,
            bottom_bar_visible: false,
            title_icon_visible: true,
            buttons: vui2::WindowDecorationButtons {
                toggle_composition: false,
                fork: false,
                minimize: false,
                restore: false,
                toggle_maximize: false,
                preserve_vm: false,
                close: true,
            },
            resize_button_visible: false,
            rotate_buttons_visible: false,
            vertical_scrollbar_visible: false,
            horizontal_scrollbar_visible: false,
            vertical_scrollbar_side: vui2::VerticalScrollbarSide::Left,
            horizontal_scrollbar_side: vui2::HorizontalScrollbarSide::Bottom,
            resize_mode: vui2::WindowResizeMode::Auto,
            resize_maintain_aspect: false,
            content_preserve_scale: true,
        },
        ..vui2::CreateOptions::default()
    };
    let window = vui2::OwnedWindow::create_with_options("Hello UI2", rect, options)?;
    let id = window.id();
    id.set_title("Hello UI2");
    Some(window)
}

pub fn create_window_1() -> Option<vui2::OwnedWindow> {
    let rect = vui2::Rect {
        x: 124,
        y: 124,
        width: 680,
        height: 420,
    };
    let options = vui2::CreateOptions {
        decorations: vui2::WindowDecorationOptions {
            mode: vui2::WindowDecorationMode::System,
            titlebar_visible: true,
            bottom_bar_visible: true,
            title_icon_visible: true,
            buttons: vui2::WindowDecorationButtons {
                toggle_composition: true,
                fork: true,
                minimize: true,
                restore: true,
                toggle_maximize: true,
                preserve_vm: true,
                close: true,
            },
            resize_button_visible: true,
            rotate_buttons_visible: false,
            vertical_scrollbar_visible: false,
            horizontal_scrollbar_visible: false,
            vertical_scrollbar_side: vui2::VerticalScrollbarSide::Left,
            horizontal_scrollbar_side: vui2::HorizontalScrollbarSide::Bottom,
            resize_mode: vui2::WindowResizeMode::Auto,
            resize_maintain_aspect: false,
            content_preserve_scale: false,
        },
        ..vui2::CreateOptions::default()
    };
    let window = vui2::OwnedWindow::create_with_options("Window 2", rect, options)?;
    let id = window.id();
    id.set_title("Window 2");
    Some(window)
}


pub fn create_all_windows() -> Vec<vui2::OwnedWindow> {
    let mut windows = Vec::new();
    if let Some(window) = create_window_0() {
        windows.push(window);
    }
    if let Some(window) = create_window_1() {
        windows.push(window);
    }
    windows
}
