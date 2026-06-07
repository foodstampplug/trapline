// Tauri v2 entry point
// CREATE_NO_WINDOW on Windows so no console flashes when spawning child processes
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    trapline_lib::run();
}
