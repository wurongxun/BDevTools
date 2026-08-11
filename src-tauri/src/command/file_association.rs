use std::os::windows::process::CommandExt;
use crate::response::resp_data::ResData;
use tauri::AppHandle;
use winreg::enums::*;
use winreg::RegKey;

const APP_PROG_ID: &str = "BackendDevelopmentTools.File";

#[tauri::command]
pub async fn register_file_association(app_handle: AppHandle) -> ResData<String> {
    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let extensions = ["md"];

    match do_register(&exe_path, &extensions) {
        Ok(_) => ResData {
            code: 200,
            msg: "注册成功".to_string(),
            data: "".to_string(),
        },
        Err(e) => ResData {
            code: 500,
            msg: format!("注册失败: {}", e),
            data: "".to_string(),
        },
    }
}

#[tauri::command]
pub async fn open_default_apps_settings() -> ResData<String> {
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "ms-settings:defaultappsbytype"])
        .creation_flags(0x08000000)
        .spawn();

    match result {
        Ok(_) => ResData {
            code: 200,
            msg: "打开设置成功".to_string(),
            data: "".to_string(),
        },
        Err(e) => ResData {
            code: 500,
            msg: format!("打开设置失败: {}", e),
            data: "".to_string(),
        },
    }
}

fn do_register(exe_path: &str, extensions: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 注册 ProgID
    let (prog_key, _) = hkcu.create_subkey(format!("Software\\Classes\\{}", APP_PROG_ID))?;
    prog_key.set_value("", &"BackendDevelopmentTools 文件")?;

    // 设置图标
    let (icon_key, _) =
        hkcu.create_subkey(format!("Software\\Classes\\{}\\DefaultIcon", APP_PROG_ID))?;
    icon_key.set_value("", &format!("\"{}\",0", exe_path))?;

    // 设置 shell\open\command
    let (cmd_key, _) =
        hkcu.create_subkey(format!("Software\\Classes\\{}\\shell\\open\\command", APP_PROG_ID))?;
    cmd_key.set_value("", &format!("\"{}\" \"%1\"", exe_path))?;

    // 为每个扩展名注册
    for ext in extensions {
        let ext_key_str = format!("Software\\Classes\\.{}", ext);
        let (ext_key, _) = hkcu.create_subkey(&ext_key_str)?;
        ext_key.set_value("", &APP_PROG_ID)?;

        // 注册到 OpenWithProgids
        let open_with_key = hkcu.open_subkey_with_flags(
            format!("Software\\Classes\\.{}\\OpenWithProgids", ext),
            KEY_WRITE,
        );
        match open_with_key {
            Ok(key) => {
                key.set_value(APP_PROG_ID, &"")?;
            }
            Err(_) => {
                let (key, _) = hkcu.create_subkey(format!(
                    "Software\\Classes\\.{}\\OpenWithProgids",
                    ext
                ))?;
                key.set_value(APP_PROG_ID, &"")?;
            }
        }
    }

    // 注册到 Applications
    let exe_name = std::path::Path::new(exe_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let (app_key, _) =
        hkcu.create_subkey(format!("Software\\Classes\\Applications\\{}", exe_name))?;
    app_key.set_value("", &"BackendDevelopmentTools")?;

    let (app_cmd_key, _) = hkcu.create_subkey(format!(
        "Software\\Classes\\Applications\\{}\\shell\\open\\command",
        exe_name
    ))?;
    app_cmd_key.set_value("", &format!("\"{}\" \"%1\"", exe_path))?;

    // 通知系统关联已更改
    notify_system_change();

    Ok(())
}
fn notify_system_change() {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, SMTO_ABORTIFHUNG,
    };
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        let setting = OsStr::new("Policy").encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let _ = SendMessageTimeoutW(
            HWND(0xffff),
            0x001A,
            WPARAM(0),
            LPARAM(setting.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
}
