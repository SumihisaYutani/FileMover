// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default)]
struct AppState {}

// Minimal commands for basic functionality
#[tauri::command]
fn hello_world() -> String {
    "Hello from Tauri!".to_string()
}

#[tauri::command]
fn get_system_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    info.insert("platform".to_string(), std::env::consts::OS.to_string());
    info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    info
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            hello_world,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}