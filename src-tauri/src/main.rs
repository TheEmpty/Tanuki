// Windows opens a console window for a program that has not said it is a
// windowed program. Tanuki is one, so it asks not to be given a terminal.
// A debug build keeps the terminal, because that is where the log goes.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    tanuki_lib::run()
}
