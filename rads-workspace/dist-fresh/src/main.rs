mod events;
mod ui;

fn main() {
    let Some(window) = ui::create_main_window() else {
        v::vshell::line("failed to create UI2 window");
        return;
    };

    events::wire_main_window();
    v::vshell::line("started Dist Fresh");

    let _window = window;
}
