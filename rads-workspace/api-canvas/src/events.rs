pub fn wire_main_window() {
    v::vshell::line("UI2 event stubs registered");
}

pub fn on_title_label_ready() {
    v::vshell::line("ready fired on titleLabel");
}

pub fn on_clear_button_click() {
    v::vshell::line("click fired on clearButton");
}

pub fn on_drawing_canvas_draw() {
    v::vshell::line("draw fired on drawingCanvas");
}
