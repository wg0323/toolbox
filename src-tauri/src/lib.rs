mod cli;
mod jdk;
mod model;
mod node;
mod ports;
mod tasks;

use tasks::Tasks;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[tauri::command]
fn app_env() -> model::AppEnv {
    model::AppEnv {
        is_admin: cli::is_admin(),
        jvms_path: cli::jvms_path().map(|p| p.to_string_lossy().to_string()),
        nvm_path: cli::nvm_path().map(|p| p.to_string_lossy().to_string()),
        java_home: std::env::var("JAVA_HOME").ok().filter(|v| !v.trim().is_empty()),
        nvm_symlink: std::env::var("NVM_SYMLINK").ok().filter(|v| !v.trim().is_empty()),
    }
}

#[tauri::command]
fn restart_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let escaped = exe.to_string_lossy().replace('\'', "''");
    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &format!("Start-Process -FilePath '{escaped}' -Verb RunAs"),
    ]);
    #[cfg(windows)]
    cmd.creation_flags(cli::CREATE_NO_WINDOW);
    cmd.spawn()
        .map_err(|e| format!("无法启动提权进程：{e}"))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("无法打开目录：{e}"))
}

#[tauri::command]
fn task_cancel(state: tauri::State<Tasks>, task_id: String) -> Result<(), String> {
    let pid = state.0.lock().unwrap().get(&task_id).copied();
    let Some(pid) = pid else {
        return Err("任务不存在或已结束".into());
    };
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/T", "/F", "/PID", &pid.to_string()]);
    #[cfg(windows)]
    cmd.creation_flags(cli::CREATE_NO_WINDOW);
    cmd.output().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Tasks::new())
        .invoke_handler(tauri::generate_handler![
            app_env,
            restart_as_admin,
            open_path,
            task_cancel,
            jdk::jdk_status,
            jdk::jdk_switch,
            jdk::jdk_remote,
            jdk::jdk_install,
            jdk::jdk_remove,
            jdk::jdk_proxy,
            node::node_status,
            node::node_switch,
            node::node_remote,
            node::node_install,
            node::node_uninstall,
            node::node_mirror,
            node::node_mirror_set,
            ports::ports_list,
            ports::port_kill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
