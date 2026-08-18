mod adapters;
mod application;
mod composition;
mod domain;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    composition::run();
}
