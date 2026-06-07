mod commands;
mod config;
mod discord;
mod findings;
mod flags;
mod runner;
mod session;

use std::sync::Mutex;
use config::Config;

/// Shared application state injected into every Tauri command handler.
/// The PID registry lives in commands::GLOBAL_PIDS (process-lifetime static)
/// so runner threads can access it without lifetime constraints.
pub struct AppState {
    pub config: Mutex<Config>,
}

pub fn run() {
    let state = AppState {
        config: Mutex::new(config::load_or_default()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::run_command,
            commands::cancel_command,
            commands::tool_check,
            commands::send_card,
            commands::send_loot,
            commands::get_config,
            commands::set_config,
            commands::test_webhook,
            commands::open_url,
            commands::save_finding,
            commands::load_findings,
            commands::delete_finding,
            commands::save_session,
            commands::load_session,
            commands::clear_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Trapline");
}
