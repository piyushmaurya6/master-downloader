pub mod commands;
pub mod core;
pub mod db;
pub mod error;

use crate::core::auth_vault::AuthVault;
use crate::core::binary_manager::{BinaryManager, ProcessRegistry};
use crate::core::engine_router::EngineRouter;
use crate::core::queue_worker::QueueWorker;
use crate::db::repository::DbRepository;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
    pub app_data_dir: PathBuf,
    pub db: DbRepository,
    pub binaries: BinaryManager,
    pub vault: AuthVault,
    pub processes: ProcessRegistry,
    pub router: EngineRouter,
    pub worker: QueueWorker,
}

pub fn resolve_app_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let primary = base.join("com.masterdownloader.desktop");
    if fs::create_dir_all(&primary).is_ok() {
        // One-time seamless migration of DB / Vault if upgrading from legacy MasterDownloader folder
        let legacy = base.join("MasterDownloader");
        if legacy.exists() {
            for filename in ["master_downloader.db", "vault.bin", ".vault_key"] {
                let src = legacy.join(filename);
                let dst = primary.join(filename);
                if src.exists() && !dst.exists() {
                    let _ = fs::copy(&src, &dst);
                }
            }
        }
        return primary;
    }
    let fallback = std::env::temp_dir().join("com.masterdownloader.desktop");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

fn percent_decode_path(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(((hi << 4) | lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn cleanup_stale_temp_files(app_data_dir: &PathBuf) {
    let tmp_dir = app_data_dir.join("tmp");
    if let Ok(entries) = fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("cookies_") || name.starts_with("gql_inspect_") {
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(&p);
                    } else {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct SingleInstanceGuard(*mut std::ffi::c_void);
#[cfg(target_os = "windows")]
unsafe impl Send for SingleInstanceGuard {}
#[cfg(target_os = "windows")]
unsafe impl Sync for SingleInstanceGuard {}
#[cfg(target_os = "windows")]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[link(name = "kernel32")]
        extern "system" {
            fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        }
        if !self.0.is_null() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn acquire_single_instance_or_focus_existing() -> Option<SingleInstanceGuard> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            lpMutexAttributes: *mut std::ffi::c_void,
            bInitialOwner: i32,
            lpName: *const u16,
        ) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut std::ffi::c_void;
        fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
        fn IsIconic(hWnd: *mut std::ffi::c_void) -> i32;
    }

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_RESTORE: i32 = 9;
    const SW_SHOW: i32 = 5;

    let mutex_name: Vec<u16> = OsStr::new("Local\\ComMasterDownloaderDesktopSingleInstance")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateMutexW(ptr::null_mut(), 0, mutex_name.as_ptr());
        if handle.is_null() {
            return Some(SingleInstanceGuard(ptr::null_mut()));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let title: Vec<u16> = OsStr::new("Master Downloader")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let hwnd = FindWindowW(ptr::null(), title.as_ptr());
            if !hwnd.is_null() {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }
                SetForegroundWindow(hwnd);
            }
            CloseHandle(handle);
            return None;
        }
        Some(SingleInstanceGuard(handle))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    let _single_instance_guard = match acquire_single_instance_or_focus_existing() {
        Some(guard) => guard,
        None => return,
    };

    let app_data_dir = resolve_app_data_dir();
    cleanup_stale_temp_files(&app_data_dir);
    let log_dir = app_data_dir.join("logs");
    let _ = fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "master-downloader.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .try_init();

    tauri::Builder::default()
        .register_uri_scheme_protocol("master-media", move |_ctx, request| {
            let uri_str = request.uri().to_string();
            let raw_path = uri_str
                .trim_start_matches("master-media://localhost/")
                .trim_start_matches("master-media://")
                .trim_start_matches("http://master-media.localhost/")
                .trim_start_matches("https://master-media.localhost/");

            let decoded = percent_decode_path(raw_path);
            let file_path = PathBuf::from(decoded);
            if file_path.exists() && file_path.is_file() {
                let within_size_limit = fs::metadata(&file_path)
                    .map(|m| m.len() <= 128 * 1024 * 1024)
                    .unwrap_or(false);
                if within_size_limit {
                    if let Ok(bytes) = fs::read(&file_path) {
                        let ext = file_path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let mime = match ext.as_str() {
                            "mp4" => "video/mp4",
                            "mkv" => "video/x-matroska",
                            "webm" => "video/webm",
                            "mp3" => "audio/mpeg",
                            "m4a" => "audio/mp4",
                            "flac" => "audio/flac",
                            "wav" => "audio/wav",
                            "opus" => "audio/ogg",
                            "png" => "image/png",
                            "webp" => "image/webp",
                            _ => "image/jpeg",
                        };
                        if let Ok(resp) = tauri::http::Response::builder()
                            .status(200)
                            .header("Content-Type", mime)
                            .header("Access-Control-Allow-Origin", "*")
                            .body(bytes)
                        {
                            return resp;
                        }
                    }
                }
            }

            tauri::http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        })
        .setup(move |app| {
            let data_dir = resolve_app_data_dir();
            let _ = fs::create_dir_all(&data_dir);

            let db_path = data_dir.join("master_downloader.db");
            let db = DbRepository::open_and_migrate(&db_path).unwrap_or_else(|err| {
                tracing::error!("Falling back to in-memory SQLite DB: {}", err);
                DbRepository::open_in_memory().expect("In-memory SQLite failed")
            });

            let bundled_bin_dir = app
                .path()
                .resource_dir()
                .ok()
                .map(|d| d.join("bin"));

            let binaries = BinaryManager::new(&data_dir, bundled_bin_dir).unwrap_or_else(|_| {
                let fallback_bin = std::env::temp_dir().join("com.masterdownloader.desktop").join("bin");
                let _ = fs::create_dir_all(&fallback_bin);
                BinaryManager {
                    bin_dir: fallback_bin,
                    bundled_bin_dir: None,
                }
            });

            let vault = AuthVault::new(&data_dir).unwrap_or_else(|_| {
                AuthVault::new(&std::env::temp_dir().join("com.masterdownloader.desktop"))
                    .expect("Temp AuthVault failed")
            });

            let processes = ProcessRegistry::new();
            let router = EngineRouter::new(
                binaries.clone(),
                vault.clone(),
                processes.clone(),
                &data_dir,
            )
            .unwrap_or_else(|_| EngineRouter {
                binaries: binaries.clone(),
                vault: vault.clone(),
                processes: processes.clone(),
                cache_dir: std::env::temp_dir(),
            });

            let worker = QueueWorker::new(db.clone(), router.clone());
            worker.spawn_background_loop(app.handle().clone());

            let state = AppState {
                app_data_dir: data_dir,
                db,
                binaries,
                vault,
                processes,
                router,
                worker,
            };

            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    if let Some(state) = window.try_state::<AppState>() {
                        state.processes.kill_all();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze::validate_and_normalize_url,
            commands::analyze::parse_batch_links,
            commands::analyze::analyze_media_url,
            commands::analyze::proxy_thumbnail_url,
            commands::queue::enqueue_job,
            commands::queue::auto_enqueue_url,
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
            commands::auth::open_facebook_login_webview,
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
        .run(tauri::generate_context!())
        .expect("error while running Master Downloader application");
}
