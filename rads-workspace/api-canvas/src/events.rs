pub fn wire_main_window() {
    v::vio::println("UI2 event stubs registered");
}

pub fn on_title_label_ready() {
    v::vio::println("ready fired on titleLabel");
}

pub fn on_clear_button_click() {
    v::vio::println("click fired on clearButton");
}

pub fn on_drawing_canvas_draw() {
    v::vio::println("draw fired on drawingCanvas");
}
