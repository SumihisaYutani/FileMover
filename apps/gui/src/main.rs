// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod error;

use tauri::Manager;
use tracing::{info, error};

use crate::state::AppState;
use crate::commands::*;

fn main() {
    // Initialize logging
    init_logging();

    info!("Starting FileMover GUI");

    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Configuration commands
            load_config,
            save_config,
            list_profiles,
            create_profile,
            delete_profile,
            
            // Scanning commands
            scan_folders,
            get_scan_progress,
            cancel_scan,
            
            // Planning commands
            create_move_plan,
            simulate_plan,
            update_plan_node,
            
            // Execution commands
            execute_plan,
            get_execution_progress,
            cancel_execution,
            
            // Undo commands
            undo_operation,
            
            // Utility commands
            browse_folder,
            validate_path,
            get_system_info
        ])
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    let log_level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        format!("filemover_gui={}", log_level)
                    )
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}