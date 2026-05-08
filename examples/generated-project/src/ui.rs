use v::vui2;

pub const APP_ID: &str = "dev.trueos.hello-ui2";
pub const APP_DISPLAY_NAME: &str = "Hello UI2";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = include_str!("../ui/index.html");
pub const MAIN_CSS: &str = include_str!("../ui/styles.css");
pub const MAIN_WINDOW_DECORATIONS: &str = "{ titlebar: true, close: true, minimize: true, maximize: true, resizable: true, always_on_top: false }";

pub fn create_main_window() -> Option<vui2::OwnedWindow> {
    let rect = vui2::Rect {
        x: 80,
        y: 80,
        width: 720,
        height: 460,
    };
    let window = vui2::OwnedWindow::create("Hello UI2", rect)?;
    let id = window.id();
    id.set_decorations(vui2::WindowDecorationMode::System);
    id.set_title("Hello UI2");
    Some(window)
}
