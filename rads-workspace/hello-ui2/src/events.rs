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

pub fn on_button1_click() {
    v::vshell::line("click fired on button1");
}

pub fn on_checkbox1_click() {
    v::vshell::line("click fired on checkbox1");
}

pub fn on_textbox1_change() {
    v::vshell::line("change fired on textbox1");
}

pub fn on_label1_ready() {
    v::vshell::line("ready fired on label1");
}
