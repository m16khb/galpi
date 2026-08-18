use crate::adapters::inbound::tauri::TauriEvents;
use crate::adapters::outbound::desktop::DesktopAdapter;
use crate::adapters::outbound::recording::NativeRecorder;
use crate::adapters::outbound::settings::LocalSettingsStore;
use crate::application::ports::{
    ArtifactPort, EnginePort, JobEvents, RecordingEvents, RecordingPort, SettingsPort,
    TranscriptionPort,
};
use crate::application::use_cases::Application;
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let events = Arc::new(TauriEvents::new(app.handle().clone()));
            let job_events: Arc<dyn JobEvents> = events.clone();
            let recording_events: Arc<dyn RecordingEvents> = events;
            let desktop = Arc::new(DesktopAdapter::new(app.handle().clone(), job_events));
            let engine: Arc<dyn EnginePort> = desktop.clone();
            let transcription: Arc<dyn TranscriptionPort> = desktop.clone();
            let artifacts: Arc<dyn ArtifactPort> = desktop;
            let recording: Arc<dyn RecordingPort> = Arc::new(NativeRecorder::new(recording_events));
            let settings: Arc<dyn SettingsPort> = Arc::new(LocalSettingsStore::new(app.handle())?);
            app.manage(Application::new(
                engine,
                transcription,
                artifacts,
                recording,
                settings,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::adapters::inbound::tauri::diagnose_environment,
            crate::adapters::inbound::tauri::prepare_environment,
            crate::adapters::inbound::tauri::load_hugging_face_token,
            crate::adapters::inbound::tauri::save_hugging_face_token,
            crate::adapters::inbound::tauri::start_transcription,
            crate::adapters::inbound::tauri::cancel_job,
            crate::adapters::inbound::tauri::open_artifact,
            crate::adapters::inbound::tauri::reveal_output_directory,
            crate::adapters::inbound::tauri::start_recording,
            crate::adapters::inbound::tauri::stop_recording,
            crate::adapters::inbound::tauri::cancel_recording,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| eprintln!("failed to run Galpi: {error}"));
}
