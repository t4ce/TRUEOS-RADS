use v::vui2;

pub const APP_ID: &str = "dev.trueos.api-canvas";
pub const APP_DISPLAY_NAME: &str = "API Canvas";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = WINDOW_0_HTML;
pub const MAIN_CSS: &str = WINDOW_0_CSS;
pub const MAIN_WINDOW_DECORATIONS: &str = WINDOW_0_DECORATIONS;
pub const WINDOW_0_MODEL: &str = include_str!("../ui/main.ui2.json");
pub const WINDOW_0_HTML: &str = include_str!("../ui/index.html");
pub const WINDOW_0_CSS: &str = include_str!("../ui/styles.css");
pub const WINDOW_0_DECORATIONS: &str = "{ titlebar: true, close: true, minimize: true, maximize: true, resizable: true, always_on_top: false }";


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
    let window = vui2::OwnedWindow::create("API Canvas", rect)?;
    let id = window.id();
    id.set_decorations(vui2::WindowDecorationMode::System);
    id.set_title("API Canvas");
    Some(window)
}


pub fn create_all_windows() -> Vec<vui2::OwnedWindow> {
    let mut windows = Vec::new();
    if let Some(window) = create_window_0() {
        windows.push(window);
    }
    windows
}
