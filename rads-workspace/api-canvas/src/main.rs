mod events;
mod ui;

pub mod lib;

fn main() {
    let windows = ui::create_all_windows();
    if windows.is_empty() {
        v::vshell::line("failed to create UI2 windows");
        return;
    }

    events::wire_main_window();
    v::vshell::line("started API Canvas");

    let _windows = windows;
}
