pub fn wire_main_window() {
    v::vshell::line("UI2 event stubs registered");
}

pub fn on_title_label_ready() {
    v::vshell::line("ready fired on titleLabel");
}

pub fn on_run_button_click() {
    v::vshell::line("click fired on runButton");
}

pub fn on_input_text_change() {
    v::vshell::line("change fired on inputText");
}
