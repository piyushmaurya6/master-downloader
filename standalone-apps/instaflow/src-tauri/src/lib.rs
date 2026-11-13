pub mod commands;
pub mod core;
pub mod db;
pub mod error;

use crate::core::auth_vault::AuthVault;
use crate::core::binary_manager::{BinaryManager, ProcessRegistry};
use crate::core::engine_router::EngineRouter;
use crate::core::queue_worker::{DiagnosticLogBuffer, QueueWorker};
use crate::db::Repository;
use std::path::PathBuf;
use tauri::RunEvent;

#[derive(Clone)]
pub struct AppState {
    pub repo: Repository,
    pub binary_manager: BinaryManager,
    pub auth_vault: AuthVault,
    pub router: EngineRouter,
    pub queue_worker: QueueWorker,
    pub process_registry: ProcessRegistry,
    pub log_buffer: DiagnosticLogBuffer,
}

fn resolve_app_data_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("InstaFlow")
}

fn ensure_bundled_binaries_seeded(target_bin_dir: & std::path::Path) {
    let _ = std::fs::create_dir_all(target_bin_dir);
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidates = [
                exe_dir.join("bin"),
                exe_dir.join("resources").join("bin"),
            ];
            for src_dir in candidates {
                if src_dir.exists() && src_dir != target_bin_dir {
                    if let Ok(entries) = std::fs::read_dir(&src_dir) {
                        for entry in entries.flatten() {
                            let dest = target_bin_dir.join(entry.file_name());
                            if !dest.exists() {
                                let _ = std::fs::copy(entry.path(), &dest);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run() {
    let data_root = resolve_app_data_root();
    let bin_dir = data_root.join("bin");
    let cache_dir = data_root.join("cache");
    let logs_dir = data_root.join("logs");
    let db_path = data_root.join("instaflow.db");

    let _ = std::fs::create_dir_all(&bin_dir);
    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::create_dir_all(&logs_dir);
    ensure_bundled_binaries_seeded(&bin_dir);

    // Initialize rolling daily log files via tracing-appender
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "instaflow.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .try_init();

    let repo = Repository::new(&db_path).expect("Failed to initialize InstaFlow SQLite database");
    let binary_manager = BinaryManager::new(bin_dir);
    let auth_vault = AuthVault::new(data_root.clone());
    let process_registry = ProcessRegistry::new();
    let log_buffer = DiagnosticLogBuffer::new();
    let router = EngineRouter::new(
        binary_manager.clone(),
        auth_vault.clone(),
        process_registry.clone(),
        cache_dir.clone(),
    );
    let queue_worker = QueueWorker::new(
        repo.clone(),
        router.clone(),
        process_registry.clone(),
        log_buffer.clone(),
    );

    let app_state = AppState {
        repo,
        binary_manager,
        auth_vault,
        router,
        queue_worker: queue_worker.clone(),
        process_registry: process_registry.clone(),
        log_buffer: log_buffer.clone(),
    };

    let cache_dir_for_protocol = cache_dir.clone();

    let app = tauri::Builder::default()
        .manage(app_state)
        .register_uri_scheme_protocol("instaflow-media", move |_ctx, request| {
            let uri_path = request.uri().path().trim_start_matches('/');
            let decoded = url::form_urlencoded::parse(uri_path.as_bytes())
                .map(|(k, v)| {
                    if v.is_empty() {
                        k.into_owned()
                    } else {
                        format!("{}={}", k, v)
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            let candidate = PathBuf::from(&decoded);
            let target_file = if candidate.is_absolute() && candidate.exists() {
                candidate
            } else {
                cache_dir_for_protocol.join("thumbs").join(&decoded)
            };

            if let Ok(bytes) = std::fs::read(&target_file) {
                let ext = target_file
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg")
                    .to_lowercase();
                let mime = match ext.as_str() {
                    "mp4" => "video/mp4",
                    "m4a" => "audio/mp4",
                    "mp3" => "audio/mpeg",
                    "png" => "image/png",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                };
                tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(bytes)
                    .unwrap_or_default()
            } else {
                tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap_or_default()
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            queue_worker.spawn_scheduler(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze::validate_and_normalize_url,
            commands::analyze::parse_batch_links,
            commands::analyze::analyze_instagram_url,
            commands::analyze::proxy_thumbnail_url,
            commands::queue::enqueue_job,
            commands::queue::enqueue_batch_jobs,
            commands::queue::list_queue,
            commands::queue::pause_job,
            commands::queue::resume_job,
            commands::queue::cancel_job,
            commands::queue::pause_all_jobs,
            commands::queue::resume_all_jobs,
            commands::queue::cancel_all_jobs,
            commands::queue::retry_failed_jobs,
            commands::queue::reorder_queue_job,
            commands::auth::get_auth_status,
            commands::auth::open_instagram_login_webview,
            commands::auth::import_cookies_txt,
            commands::auth::set_browser_cookie_mode,
            commands::auth::logout_and_wipe_vault,
            commands::binaries::check_binaries_status,
            commands::binaries::bootstrap_or_update_binaries,
            commands::binaries::verify_binary_sha256,
            commands::history::query_download_history,
            commands::history::delete_history_item,
            commands::history::reveal_in_explorer,
            commands::history::open_media_file,
            commands::history::export_history_json,
            commands::settings::get_app_settings,
            commands::settings::update_app_settings,
            commands::settings::pick_directory_dialog,
            commands::settings::get_diagnostic_logs,
            commands::settings::export_diagnostic_bundle,
        ])
        .build(tauri::generate_context!())
        .expect("Error while building InstaFlow Tauri application");

    app.run(move |_app_handle, event| match event {
        RunEvent::ExitRequested { .. } | RunEvent::Exit => {
            process_registry.kill_all();
        }
        _ => {}
    });
}
