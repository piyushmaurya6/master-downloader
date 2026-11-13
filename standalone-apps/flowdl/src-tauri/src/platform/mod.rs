use crate::utils::no_window_command;
use std::path::Path;

#[cfg(target_os = "windows")]
mod win32_dialog {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::ptr;

    #[repr(C)]
    struct BROWSEINFOW {
        hwnd_owner: *mut std::ffi::c_void,
        pidl_root: *const std::ffi::c_void,
        psz_display_name: *mut u16,
        lpsz_title: *const u16,
        ul_flags: u32,
        lpfn: Option<unsafe extern "system" fn(*mut std::ffi::c_void, u32, isize, isize) -> i32>,
        l_param: isize,
        i_image: i32,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHBrowseForFolderW(lpbi: *mut BROWSEINFOW) -> *mut std::ffi::c_void;
        fn SHGetPathFromIDListW(pidl: *const std::ffi::c_void, psz_path: *mut u16) -> i32;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pv_reserved: *mut std::ffi::c_void, dw_co_init: u32) -> i32;
        fn CoTaskMemFree(pv: *mut std::ffi::c_void);
    }

    #[link(name = "user32")]
    extern "system" {
        fn SendMessageW(
            hwnd: *mut std::ffi::c_void,
            msg: u32,
            wparam: usize,
            lparam: isize,
        ) -> isize;
    }

    const BIF_RETURNONLYFSDIRS: u32 = 0x00000001;
    const BIF_NEWDIALOGSTYLE: u32 = 0x00000040;
    const BIF_USENEWUI: u32 = BIF_NEWDIALOGSTYLE | 0x00000010;
    const BFFM_INITIALIZED: u32 = 1;
    const BFFM_SETSELECTIONW: u32 = 0x0400 + 103;
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    unsafe extern "system" fn browse_callback(
        hwnd: *mut std::ffi::c_void,
        u_msg: u32,
        _l_param: isize,
        lp_data: isize,
    ) -> i32 {
        if u_msg == BFFM_INITIALIZED && lp_data != 0 {
            SendMessageW(hwnd, BFFM_SETSELECTIONW, 1, lp_data);
        }
        0
    }

    pub fn pick_folder(initial_dir: Option<String>) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let mut display_name = [0u16; 260];
            let title: Vec<u16> = std::ffi::OsStr::new("Select FlowDL Download Folder")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let init_wide: Option<Vec<u16>> = initial_dir.and_then(|d| {
                if d.trim().is_empty() || !std::path::Path::new(&d).exists() {
                    None
                } else {
                    Some(
                        std::ffi::OsStr::new(&d)
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect(),
                    )
                }
            });

            let mut bi = BROWSEINFOW {
                hwnd_owner: ptr::null_mut(),
                pidl_root: ptr::null(),
                psz_display_name: display_name.as_mut_ptr(),
                lpsz_title: title.as_ptr(),
                ul_flags: BIF_RETURNONLYFSDIRS | BIF_USENEWUI,
                lpfn: Some(browse_callback),
                l_param: init_wide.as_ref().map(|v| v.as_ptr() as isize).unwrap_or(0),
                i_image: 0,
            };

            let pidl = SHBrowseForFolderW(&mut bi);
            if pidl.is_null() {
                return None;
            }
            let mut path_buf = [0u16; 1024];
            let ok = SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr());
            CoTaskMemFree(pidl);
            if ok != 0 {
                let len = path_buf.iter().position(|&c| c == 0).unwrap_or(0);
                let os_str = OsString::from_wide(&path_buf[..len]);
                Some(os_str.to_string_lossy().to_string())
            } else {
                None
            }
        }
    }
}

pub fn pick_folder_dialog(initial_dir: Option<String>) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        win32_dialog::pick_folder(initial_dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        initial_dir
    }
}

pub fn open_file_in_os(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("The file no longer exists at its saved location.".to_string());
    }
    no_window_command("explorer.exe")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;
    Ok(())
}

pub fn reveal_in_explorer(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if p.is_file() && p.exists() {
        no_window_command("explorer.exe")
            .arg(format!("/select,{}", path))
            .spawn()
            .map_err(|e| format!("Failed to reveal file in Explorer: {}", e))?;
    } else if p.is_dir() && p.exists() {
        no_window_command("explorer.exe")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder in Explorer: {}", e))?;
    } else if let Some(parent) = p.parent() {
        if parent.exists() {
            no_window_command("explorer.exe")
                .arg(parent.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| format!("Failed to open folder in Explorer: {}", e))?;
        } else {
            return Err("Folder does not exist.".to_string());
        }
    } else {
        return Err("Target path does not exist.".to_string());
    }
    Ok(())
}

/// Sanitizes a filename or template segment against Windows reserved characters, control chars,
/// path traversal sequences (`..`), and reserved DOS device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`).
pub fn sanitize_windows_filename(raw: &str) -> String {
    let mut cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect();

    // Prevent relative path traversal segments
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", "_");
    }

    cleaned = cleaned.trim().trim_end_matches('.').to_string();
    if cleaned.is_empty() {
        return "download".to_string();
    }

    let stem_upper = cleaned
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&stem_upper.as_str()) {
        cleaned = format!("_{}", cleaned);
    }

    cleaned
}

/// Validates that an output directory path is safe and does not contain traversal outside the intended path.
pub fn validate_safe_output_dir(dir_str: &str) -> Result<String, String> {
    let trimmed = dir_str.trim();
    if trimmed.is_empty() {
        return Err("Output directory cannot be empty.".to_string());
    }
    if trimmed.contains("..") {
        return Err("Output directory cannot contain relative path traversal ('..').".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn os_version_summary() -> String {
    format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_windows_filename_protects_reserved_and_traversal() {
        assert_eq!(
            sanitize_windows_filename("../../CON.mp4"),
            "____CON.mp4"
        );
        assert_eq!(sanitize_windows_filename("CON.mp4"), "_CON.mp4");
        assert_eq!(
            sanitize_windows_filename("My:Video*Title?.mp4."),
            "My_Video_Title_.mp4"
        );
        assert_eq!(
            sanitize_windows_filename("Unicode 🎵 测试 Video.mp4"),
            "Unicode 🎵 测试 Video.mp4"
        );
    }

    #[test]
    fn test_validate_safe_output_dir() {
        assert!(validate_safe_output_dir("C:\\Users\\Test\\Downloads").is_ok());
        assert!(validate_safe_output_dir("C:\\Users\\..\\System32").is_err());
    }
}
