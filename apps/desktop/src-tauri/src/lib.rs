use rtc_domain::{
    Capabilities, ImportIssue, PeerConnectionSummary, RtcSession, SessionAnalysis, SessionSummary,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct AppState {
    current_session: Mutex<Option<(String, RtcSession)>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    app_name: &'static str,
    version: &'static str,
    platform: &'static str,
}

#[tauri::command]
fn runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        app_name: "RTC Inspector",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportDumpResult {
    session_id: String,
    summary: SessionSummary,
    peer_connections: Vec<PeerConnectionSummary>,
    capabilities: Capabilities,
    issues: Vec<ImportIssue>,
    analysis: SessionAnalysis,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportDumpError {
    code: String,
    message: String,
}

#[tauri::command]
async fn import_dump(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<ImportDumpResult, ImportDumpError> {
    let source_path = PathBuf::from(path);
    let session =
        tauri::async_runtime::spawn_blocking(move || dump_reader::parse_path(&source_path))
            .await
            .map_err(|error| ImportDumpError {
                code: "dump.parser-task-failed".into(),
                message: error.to_string(),
            })?
            .map_err(|error| ImportDumpError {
                code: error.code().into(),
                message: error.to_string(),
            })?;

    let result = ImportDumpResult {
        session_id: Uuid::new_v4().to_string(),
        summary: session.summary(),
        peer_connections: session
            .peer_connections
            .iter()
            .map(PeerConnectionSummary::from)
            .collect(),
        capabilities: session.capabilities.clone(),
        issues: session.issues.clone(),
        analysis: session.analysis(),
    };
    let mut current = state.current_session.lock().map_err(|_| ImportDumpError {
        code: "dump.session-store-failed".into(),
        message: "failed to store the parsed session".into(),
    })?;
    *current = Some((result.session_id.clone(), session));
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![runtime_status, import_dump])
        .run(tauri::generate_context!())
        .expect("failed to run RTC Inspector");
}
