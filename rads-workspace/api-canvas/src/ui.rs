use v::vui2;

pub const APP_ID: &str = "dev.trueos.api-canvas";
pub const APP_DISPLAY_NAME: &str = "API Canvas";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = include_str!("../ui/index.html");
pub const MAIN_CSS: &str = include_str!("../ui/styles.css");
pub const MAIN_WINDOW_DECORATIONS: &str = "{ titlebar: true, close: true, minimize: true, maximize: true, resizable: true, always_on_top: false }";

pub fn create_main_window() -> Option<vui2::OwnedWindow> {
    let rect = vui2::Rect {
        x: 60,
        y: 60,
        w: 960,
        h: 640,
    };
    let window = vui2::OwnedWindow::create("API Canvas", rect)?;
    window
        .set_decorations(vui2::WindowDecorationMode::System)
        .set_title("API Canvas");
    Some(window)
}
