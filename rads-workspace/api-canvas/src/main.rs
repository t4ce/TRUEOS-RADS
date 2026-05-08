mod events;
mod ui;

fn main() {
    let Some(window) = ui::create_main_window() else {
        v::vio::println("failed to create UI2 window");
        return;
    };

    events::wire_main_window();
    v::vio::println("started API Canvas");

    let _window = window;
}
