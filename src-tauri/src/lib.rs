mod commands;
mod config;
mod deck;
mod discord;
mod findings;
mod flags;
mod integrations;
mod runner;
mod session;
mod watch;

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
            deck::deck_start,
            deck::deck_stop,
            deck::deck_status,
            deck::deck_set_folder,
            watch::scheduler::watch_start,
            watch::scheduler::watch_stop,
            watch::scheduler::watch_status,
            watch::scheduler::watch_run_once,
            integrations::commands::shodan_host,
            integrations::commands::shodan_domain,
            integrations::commands::shodan_search,
            integrations::commands::leakcheck_domain,
            integrations::commands::leakcheck_email,
        ])
        .setup(|app| {
            use tauri::Manager;
            let enabled = app.state::<AppState>().config.lock().unwrap().watch_enabled;
            if enabled {
                watch::scheduler::start(app.handle().clone());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Trapline")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                watch::scheduler::stop_flag();
                let _ = deck::deck_stop();
            }
        });
}
