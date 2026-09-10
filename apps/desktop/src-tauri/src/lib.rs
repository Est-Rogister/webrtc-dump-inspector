use serde::Serialize;
use session_store::{SessionStore, SourceError, StoreErrorDto, StoredSource, WorkspaceSessionDto};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const DEFAULT_MAX_STATS_POINTS: usize = 2_000;

#[derive(Clone, Default)]
struct AppState {
    sessions: Arc<Mutex<SessionStore>>,
    jobs: Arc<Mutex<HashMap<String, ImportJob>>>,
    parse_gate: Arc<Mutex<()>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    app_name: &'static str,
    version: &'static str,
    platform: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ImportJobStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportJobSnapshot {
    id: String,
    status: ImportJobStatus,
    completed: usize,
    total: usize,
    current_file: Option<String>,
    session_id: Option<String>,
    errors: Vec<SourceError>,
}

struct ImportJob {
    snapshot: ImportJobSnapshot,
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
fn runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        app_name: "RTC Inspector",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    }
}

#[tauri::command]
fn start_import(
    paths: Vec<String>,
    target_session_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<ImportJobSnapshot, StoreErrorDto> {
    if paths.is_empty() {
        return Err(error("import.no-files", "no dump files were selected"));
    }

    let id = Uuid::new_v4().to_string();
    let snapshot = ImportJobSnapshot {
        id: id.clone(),
        status: ImportJobStatus::Queued,
        completed: 0,
        total: paths.len(),
        current_file: None,
        session_id: None,
        errors: Vec::new(),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    state.jobs.lock().map_err(|_| lock_error())?.insert(
        id.clone(),
        ImportJob {
            snapshot: snapshot.clone(),
            cancel: Arc::clone(&cancel),
        },
    );

    let sessions = Arc::clone(&state.sessions);
    let jobs = Arc::clone(&state.jobs);
    let parse_gate = Arc::clone(&state.parse_gate);
    let job_id = id;
    tauri::async_runtime::spawn_blocking(move || {
        run_import_job(
            &job_id,
            paths,
            target_session_id,
            cancel,
            sessions,
            jobs,
            parse_gate,
        );
    });

    Ok(snapshot)
}

fn run_import_job(
    job_id: &str,
    paths: Vec<String>,
    target_session_id: Option<String>,
    cancel: Arc<AtomicBool>,
    sessions: Arc<Mutex<SessionStore>>,
    jobs: Arc<Mutex<HashMap<String, ImportJob>>>,
    parse_gate: Arc<Mutex<()>>,
) {
    let Ok(_permit) = parse_gate.lock() else {
        fail_job(&jobs, job_id, lock_error());
        return;
    };
    update_job(&jobs, job_id, |snapshot| {
        snapshot.status = ImportJobStatus::Running;
    });

    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        update_job(&jobs, job_id, |snapshot| {
            snapshot.current_file = Some(file_name(&path));
        });
        match dump_reader::parse_path(Path::new(&path)) {
            Ok(session) => sources.push(StoredSource::new(path.clone(), session)),
            Err(parse_error) => errors.push(SourceError {
                path: path.clone(),
                name: file_name(&path),
                error: StoreErrorDto {
                    code: parse_error.code().into(),
                    message: parse_error.to_string(),
                },
            }),
        }
        update_job(&jobs, job_id, |snapshot| {
            snapshot.completed += 1;
            snapshot.errors = errors.clone();
        });
    }

    let session_id = if sources.is_empty() {
        None
    } else {
        match sessions.lock() {
            Ok(mut store) => {
                match store.add_sources(target_session_id.as_deref(), sources, errors.clone()) {
                    Ok(value) => value,
                    Err(store_error) => {
                        fail_job(&jobs, job_id, store_error);
                        return;
                    }
                }
            }
            Err(_) => {
                fail_job(&jobs, job_id, lock_error());
                return;
            }
        }
    };

    update_job(&jobs, job_id, |snapshot| {
        snapshot.current_file = None;
        snapshot.session_id = session_id;
        snapshot.errors = errors;
        snapshot.status = if cancel.load(Ordering::Relaxed) {
            ImportJobStatus::Cancelled
        } else if snapshot.session_id.is_some() {
            ImportJobStatus::Completed
        } else {
            ImportJobStatus::Failed
        };
    });
}

fn update_job(
    jobs: &Arc<Mutex<HashMap<String, ImportJob>>>,
    job_id: &str,
    update: impl FnOnce(&mut ImportJobSnapshot),
) {
    if let Ok(mut jobs) = jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            update(&mut job.snapshot);
        }
    }
}

fn fail_job(jobs: &Arc<Mutex<HashMap<String, ImportJob>>>, job_id: &str, failure: StoreErrorDto) {
    update_job(jobs, job_id, |snapshot| {
        snapshot.status = ImportJobStatus::Failed;
        snapshot.current_file = None;
        snapshot.errors.push(SourceError {
            path: String::new(),
            name: "import".into(),
            error: failure,
        });
    });
}

#[tauri::command]
fn get_import_job(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ImportJobSnapshot, StoreErrorDto> {
    state
        .jobs
        .lock()
        .map_err(|_| lock_error())?
        .get(&job_id)
        .map(|job| job.snapshot.clone())
        .ok_or_else(|| {
            error(
                "import.job-not-found",
                format!("import job {job_id} was not found"),
            )
        })
}

#[tauri::command]
fn cancel_import(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ImportJobSnapshot, StoreErrorDto> {
    let mut jobs = state.jobs.lock().map_err(|_| lock_error())?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        error(
            "import.job-not-found",
            format!("import job {job_id} was not found"),
        )
    })?;
    job.cancel.store(true, Ordering::Relaxed);
    if job.snapshot.status == ImportJobStatus::Queued {
        job.snapshot.status = ImportJobStatus::Cancelled;
    }
    Ok(job.snapshot.clone())
}

#[tauri::command]
fn list_sessions(
    max_stats_points: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WorkspaceSessionDto>, StoreErrorDto> {
    let maximum = normalize_max_points(max_stats_points);
    Ok(state
        .sessions
        .lock()
        .map_err(|_| lock_error())?
        .list(maximum))
}

#[tauri::command]
fn get_session(
    session_id: String,
    max_stats_points: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<WorkspaceSessionDto, StoreErrorDto> {
    state
        .sessions
        .lock()
        .map_err(|_| lock_error())?
        .get(&session_id, normalize_max_points(max_stats_points))
        .ok_or_else(|| {
            error(
                "session.not-found",
                format!("session {session_id} was not found"),
            )
        })
}

#[tauri::command]
fn close_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<bool, StoreErrorDto> {
    Ok(state
        .sessions
        .lock()
        .map_err(|_| lock_error())?
        .remove(&session_id))
}

fn normalize_max_points(value: Option<usize>) -> usize {
    value.unwrap_or(DEFAULT_MAX_STATS_POINTS).clamp(100, 10_000)
}

fn file_name(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn lock_error() -> StoreErrorDto {
    error("app.state-unavailable", "application state is unavailable")
}

fn error(code: impl Into<String>, message: impl Into<String>) -> StoreErrorDto {
    StoreErrorDto {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            runtime_status,
            start_import,
            get_import_job,
            cancel_import,
            list_sessions,
            get_session,
            close_session,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RTC Inspector");
}

#[cfg(test)]
mod tests {
    use super::{run_import_job, AppState, ImportJob, ImportJobSnapshot, ImportJobStatus};
    use std::fs;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempDump(std::path::PathBuf);

    impl TempDump {
        fn new(contents: &[u8]) -> Self {
            let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rtc-inspector-job-{}-{id}.jsonl",
                std::process::id()
            ));
            fs::write(&path, contents).unwrap();
            Self(path)
        }

        fn path(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempDump {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn install_job(state: &AppState, job_id: &str, total: usize, cancel: Arc<AtomicBool>) {
        state.jobs.lock().unwrap().insert(
            job_id.into(),
            ImportJob {
                snapshot: ImportJobSnapshot {
                    id: job_id.into(),
                    status: ImportJobStatus::Queued,
                    completed: 0,
                    total,
                    current_file: None,
                    session_id: None,
                    errors: vec![],
                },
                cancel,
            },
        );
    }

    #[test]
    fn batch_import_keeps_successful_sources_when_another_file_fails() {
        let state = AppState::default();
        let valid = TempDump::new(b"RTCStatsDump\n{\"fileFormat\":3}\n[39,\"pc\",{},1]\n");
        let invalid = TempDump::new(b"not a dump");
        let cancel = Arc::new(AtomicBool::new(false));
        install_job(&state, "job", 2, Arc::clone(&cancel));

        run_import_job(
            "job",
            vec![valid.path(), invalid.path()],
            None,
            cancel,
            Arc::clone(&state.sessions),
            Arc::clone(&state.jobs),
            Arc::clone(&state.parse_gate),
        );

        let jobs = state.jobs.lock().unwrap();
        let snapshot = &jobs["job"].snapshot;
        assert_eq!(snapshot.status, ImportJobStatus::Completed);
        assert_eq!(snapshot.completed, 2);
        assert_eq!(snapshot.errors.len(), 1);
        let session_id = snapshot.session_id.as_ref().unwrap();
        let session = state
            .sessions
            .lock()
            .unwrap()
            .get(session_id, 2_000)
            .unwrap();
        assert_eq!(session.summary.source_count, 1);
        assert_eq!(session.source_errors.len(), 1);
    }

    #[test]
    fn pre_cancelled_job_does_not_parse_or_create_a_session() {
        let state = AppState::default();
        let valid = TempDump::new(b"RTCStatsDump\n{\"fileFormat\":3}\n[39,\"pc\",{},1]\n");
        let cancel = Arc::new(AtomicBool::new(true));
        install_job(&state, "job", 1, Arc::clone(&cancel));

        run_import_job(
            "job",
            vec![valid.path()],
            None,
            cancel,
            Arc::clone(&state.sessions),
            Arc::clone(&state.jobs),
            Arc::clone(&state.parse_gate),
        );

        let jobs = state.jobs.lock().unwrap();
        assert_eq!(jobs["job"].snapshot.status, ImportJobStatus::Cancelled);
        assert_eq!(jobs["job"].snapshot.completed, 0);
        assert!(jobs["job"].snapshot.session_id.is_none());
        assert!(state.sessions.lock().unwrap().list(100).is_empty());
    }
}
