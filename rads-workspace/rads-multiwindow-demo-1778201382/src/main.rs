mod events;
mod ui;

fn main() {
    let windows = ui::create_all_windows();
    if windows.is_empty() {
        v::vshell::line("failed to create UI2 windows");
        return;
    }

    events::wire_main_window();
    v::vshell::line("started RADS Multiwindow Demo 1778201382");

    let _windows = windows;
}
