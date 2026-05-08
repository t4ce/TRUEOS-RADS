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
pub const WINDOW_0_DECORATIONS: &str = "{ titlebar: true, close: true, minimize: true, maximize: true, resizable: true, always_on_top: false }";
pub const WINDOW_1_MODEL: &str = include_str!("../ui/windows/window2.ui2.json");
pub const WINDOW_1_HTML: &str = include_str!("../ui/windows/window2.html");
pub const WINDOW_1_CSS: &str = include_str!("../ui/windows/window2.css");
pub const WINDOW_1_DECORATIONS: &str = "{ titlebar: true, close: true, minimize: true, maximize: true, resizable: true, always_on_top: false }";


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
    let window = vui2::OwnedWindow::create("Hello UI2", rect)?;
    let id = window.id();
    id.set_decorations(vui2::WindowDecorationMode::System);
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
    let window = vui2::OwnedWindow::create("Window 2", rect)?;
    let id = window.id();
    id.set_decorations(vui2::WindowDecorationMode::System);
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
