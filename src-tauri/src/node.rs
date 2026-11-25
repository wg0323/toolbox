use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::cli;
use crate::model::{CliOutput, EnvStatus, RemoteVersion, VersionItem, VersionPage};
use crate::tasks;

fn nvm_root() -> Option<PathBuf> {
    cli::nvm_path()?.parent().map(|p| p.to_path_buf())
}

fn read_settings() -> (Option<String>, Option<String>) {
    let Some(root) = nvm_root() else {
        return (None, None);
    };
    let text = std::fs::read_to_string(root.join("settings.txt")).unwrap_or_default();
    let mut root_path = None;
    let mut symlink = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("root:") {
            root_path = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("path:") {
            symlink = Some(value.trim().to_string());
        }
    }
    (root_path, symlink)
}

fn parse_list(stdout: &str) -> Vec<VersionItem> {
    let mut items = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("No installations") {
            continue;
        }
        let active = trimmed.starts_with('*');
        let body = trimmed.trim_start_matches('*').trim();
        let Some(first) = body.split_whitespace().next() else { continue };
        let version = first.trim_start_matches('v');
        if version.is_empty() || !version.chars().next().unwrap().is_ascii_digit() {
            continue;
        }
        items.push(VersionItem {
            version: version.to_string(),
            active,
            path: None,
        });
    }
    items
}

fn probe_node() -> Option<String> {
    let node = cli::find_tool("node.exe", &[])?;
    let out = cli::run(&node, &["-v"], Duration::from_secs(10)).ok()?;
    let text = out.stdout.trim().trim_start_matches('v');
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .trim_start_matches('v')
        .trim_end_matches(['\\', '/'])
        .to_string()
}

fn link_last_segment(target: &str) -> &str {
    target.rsplit(['\\', '/']).next().unwrap_or(target)
}

pub fn load_status() -> VersionPage {
    let mut warnings = Vec::new();
    let mut error = None;
    let nvm = cli::nvm_path();

    let mut versions = Vec::new();
    if let Some(ref tool) = nvm {
        match cli::run(tool, &["list"], Duration::from_secs(12)) {
            Ok(out) if out.success => versions = parse_list(&out.stdout),
            Ok(out) => error = Some(cli::failure_text(&out)),
            Err(e) => error = Some(e),
        }
    } else {
        error = Some("未找到 nvm.exe，请确认它已加入 PATH".to_string());
    }

    let (root, symlink) = read_settings();
    let link_path = symlink
        .or_else(|| std::env::var("NVM_SYMLINK").ok())
        .unwrap_or_else(|| "C:\\nvm4w\\nodejs".to_string());
    let link_target = cli::read_link(&link_path).map(|target| target.trim_start_matches(r"\\?\").to_string());

    let probe = probe_node();
    let active = versions.iter().find(|item| item.active).map(|item| item.version.clone());

    if link_target.is_none() {
        warnings.push(format!("无法读取符号链接：{link_path}"));
    }
    match (&active, &link_target) {
        (Some(active), Some(target)) if normalize(active) != normalize(link_last_segment(target)) => {
            warnings.push(format!(
                "nvm 标记的当前版本 {active} 与链接目标 {} 不一致",
                link_last_segment(target)
            ));
        }
        (None, _) => warnings.push("nvm 未能识别当前版本".to_string()),
        _ => {}
    }
    match (&probe, &link_target) {
        (Some(probe), Some(target)) if normalize(probe) != normalize(link_last_segment(target)) => {
            warnings.push(format!(
                "PATH 中的 node（{probe}）不是链接目标 {}，切换不会生效",
                link_last_segment(target)
            ));
        }
        (None, _) => warnings.push("执行 node -v 失败，PATH 中可能没有 node".to_string()),
        _ => {}
    }

    let _ = root;
    let consistent = warnings.is_empty();

    VersionPage {
        status: EnvStatus {
            tool_path: nvm.map(|p| p.to_string_lossy().to_string()),
            current: active,
            link_path: Some(link_path),
            link_target,
            probe,
            consistent,
            warnings,
        },
        versions,
        error,
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[tauri::command]
pub fn node_status() -> VersionPage {
    load_status()
}

#[tauri::command]
pub fn node_switch(version: String) -> Result<CliOutput, String> {
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    let out = cli::run(&nvm, &["use", &version], Duration::from_secs(40))?;
    if !out.success {
        return Err(format!(
            "{}。nvm use 需要替换符号链接，非管理员运行时 nvm 会弹出 UAC，取消或占用文件都会失败",
            cli::failure_text(&out)
        ));
    }
    Ok(out)
}

fn parse_available(stdout: &str, installed: &[String]) -> Vec<RemoteVersion> {
    let groups = ["CURRENT", "LTS", "OLD STABLE", "OLD UNSTABLE"];
    let mut items: Vec<RemoteVersion> = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || trimmed.contains("---") || trimmed.contains("CURRENT") {
            continue;
        }
        let cells: Vec<&str> = trimmed.trim_matches('|').split('|').collect();
        for (index, cell) in cells.iter().enumerate() {
            let version = cell.trim();
            if version.is_empty() || !version.contains('.') {
                continue;
            }
            if !version
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                continue;
            }
            if items.iter().any(|item| item.version == version) {
                continue;
            }
            items.push(RemoteVersion {
                version: version.to_string(),
                group: groups.get(index).map(|g| g.to_string()),
                installed: installed.iter().any(|v| v == version),
            });
        }
    }
    items
}

#[tauri::command]
pub fn node_remote() -> Result<Vec<RemoteVersion>, String> {
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    let out = cli::run(&nvm, &["list", "available"], Duration::from_secs(60))?;
    if !out.success {
        return Err(cli::failure_text(&out));
    }

    let installed: Vec<String> = load_status()
        .versions
        .into_iter()
        .map(|item| item.version)
        .collect();
    Ok(parse_available(&out.stdout, &installed))
}

#[tauri::command]
pub fn node_install(app: AppHandle, version: String) -> Result<String, String> {
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    let task_id = format!("node-install-{}-{version}", now_millis());
    tasks::spawn(app, task_id.clone(), nvm, vec!["install".into(), version])?;
    Ok(task_id)
}

#[tauri::command]
pub fn node_uninstall(version: String) -> Result<CliOutput, String> {
    let page = load_status();
    if page.status.current.as_deref() == Some(version.as_str()) {
        return Err("当前正在使用的版本不能卸载，请先切换到其他版本".into());
    }
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    let out = cli::run(&nvm, &["uninstall", &version], Duration::from_secs(60))?;
    if !out.success {
        return Err(cli::failure_text(&out));
    }
    Ok(out)
}

#[tauri::command]
pub fn node_mirror() -> Result<(String, String), String> {
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    let node = cli::run(&nvm, &["node_mirror"], Duration::from_secs(10))?;
    let npm = cli::run(&nvm, &["npm_mirror"], Duration::from_secs(10))?;
    Ok((node.stdout.trim().to_string(), npm.stdout.trim().to_string()))
}

#[tauri::command]
pub fn node_mirror_set(node_url: String, npm_url: String) -> Result<(), String> {
    let nvm = cli::nvm_path().ok_or("未找到 nvm.exe")?;
    if !node_url.trim().is_empty() {
        let out = cli::run(&nvm, &["node_mirror", node_url.trim()], Duration::from_secs(15))?;
        if !out.success {
            return Err(cli::failure_text(&out));
        }
    }
    if !npm_url.trim().is_empty() {
        let out = cli::run(&nvm, &["npm_mirror", npm_url.trim()], Duration::from_secs(15))?;
        if !out.success {
            return Err(cli::failure_text(&out));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvm_list() {
        let text = "\n    24.14.1\n  * 22.22.2 (Currently using 64-bit executable)\n    20.20.2\n";
        let items = parse_list(text);
        assert_eq!(items.len(), 3);
        assert!(items[1].active);
        assert_eq!(items[1].version, "22.22.2");
        assert_eq!(items[0].version, "24.14.1");
    }

    #[test]
    fn parses_available_table() {
        let text = "|      CURRENT       |     LTS      |  OLD STABLE  | OLD UNSTABLE |\n|--------------------|--------------|--------------|--------------|\n| 27.0.0-alpha.0.1.1 |   26.13.1    |   0.12.18    |   0.11.16    |\n";
        let installed = vec!["26.13.1".to_string()];
        let items = parse_available(text, &installed);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].version, "27.0.0-alpha.0.1.1");
        assert_eq!(items[0].group.as_deref(), Some("CURRENT"));
        assert!(items[1].installed);
        assert_eq!(items[1].group.as_deref(), Some("LTS"));
    }

    #[test]
    fn normalizes_version_prefix() {
        assert_eq!(normalize("v22.22.2"), "22.22.2");
        assert_eq!(normalize("  V18.20.7 "), "18.20.7");
    }
}
