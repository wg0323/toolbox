use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::cli;
use crate::model::{CliOutput, EnvStatus, RemoteVersion, VersionItem, VersionPage};
use crate::tasks;

fn store_dir() -> Option<PathBuf> {
    let jvms = cli::jvms_path()?;
    Some(jvms.parent()?.join("store"))
}

fn parse_list(stdout: &str) -> Vec<VersionItem> {
    let mut items = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Installed") || trimmed.starts_with("No installations") {
            continue;
        }
        let active = trimmed.starts_with('*');
        let body = trimmed.trim_start_matches('*').trim();
        let Some(close) = body.find(')') else { continue };
        let version = body[close + 1..].trim().to_string();
        if version.is_empty() {
            continue;
        }
        let path = store_dir()
            .map(|dir| dir.join(&version))
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy().to_string());
        items.push(VersionItem { version, active, path });
    }
    items
}

fn parse_java_version(text: &str) -> Option<String> {
    for line in text.lines() {
        if !line.to_ascii_lowercase().contains("version") {
            continue;
        }
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn probe_java(link_path: &str) -> Option<String> {
    let java = cli::find_tool(
        "java.exe",
        &[format!("{link_path}\\bin\\java.exe")],
    )?;
    let out = cli::run(&java, &["-version"], Duration::from_secs(10)).ok()?;
    let text = if out.stdout.trim().is_empty() {
        out.stderr
    } else {
        out.stdout
    };
    parse_java_version(&text)
}

fn same_version(left: &str, right: &str) -> bool {
    let a = left.trim().to_ascii_lowercase();
    let b = right.trim().to_ascii_lowercase();
    a == b || a.starts_with(&b) || b.starts_with(&a)
}

fn link_last_segment(target: &str) -> &str {
    target.rsplit(['\\', '/']).next().unwrap_or(target)
}

pub fn load_status() -> VersionPage {
    let mut warnings = Vec::new();
    let mut error = None;
    let jvms = cli::jvms_path();

    let mut versions = Vec::new();
    if let Some(ref tool) = jvms {
        match cli::run(tool, &["list"], Duration::from_secs(12)) {
            Ok(out) if out.success => versions = parse_list(&out.stdout),
            Ok(out) => error = Some(cli::failure_text(&out)),
            Err(e) => error = Some(e),
        }
    } else {
        error = Some("未找到 jvms.exe，请确认它已加入 PATH".to_string());
    }

    let link_path = std::env::var("JAVA_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "C:\\Program Files\\jdk".to_string());
    let link_target = cli::read_link(&link_path).map(|target| {
        target
            .trim_start_matches(r"\\?\")
            .to_string()
    });

    let probe = probe_java(&link_path);
    let active = versions.iter().find(|item| item.active).map(|item| item.version.clone());

    if link_target.is_none() {
        warnings.push(format!("无法读取符号链接：{link_path}"));
    }
    match (&active, &link_target) {
        (Some(active), Some(target)) if !same_version(active, link_last_segment(target)) => {
            warnings.push(format!(
                "jvms 标记的当前版本 {active} 与链接目标 {} 不一致",
                link_last_segment(target)
            ));
        }
        (Some(_), None) => {}
        (None, _) => warnings.push("jvms 未能识别当前版本".to_string()),
        _ => {}
    }
    match (&probe, &link_target) {
        (Some(probe), Some(target)) if !same_version(probe, link_last_segment(target)) => {
            warnings.push(format!(
                "PATH 中的 java（{probe}）不是链接目标 {}，切换不会生效",
                link_last_segment(target)
            ));
        }
        (None, _) => warnings.push("执行 java -version 失败，PATH 中可能没有 java".to_string()),
        _ => {}
    }

    let consistent = warnings.is_empty();

    VersionPage {
        status: EnvStatus {
            tool_path: jvms.map(|p| p.to_string_lossy().to_string()),
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

fn require_admin(action: &str) -> Result<(), String> {
    if cli::is_admin() {
        Ok(())
    } else {
        Err(format!(
            "{action}需要管理员权限：jvms 会改写系统 JAVA_HOME 与 C:\\Program Files\\jdk 链接。请点击右上角以管理员身份重启。"
        ))
    }
}

#[tauri::command]
pub fn jdk_status() -> VersionPage {
    load_status()
}

#[tauri::command]
pub fn jdk_switch(version: String) -> Result<CliOutput, String> {
    require_admin("切换 JDK")?;
    let jvms = cli::jvms_path().ok_or("未找到 jvms.exe")?;
    let out = cli::run(&jvms, &["switch", &version], Duration::from_secs(20))?;
    if !out.success {
        return Err(cli::failure_text(&out));
    }
    Ok(out)
}

#[tauri::command]
pub fn jdk_remote(all: bool) -> Result<Vec<RemoteVersion>, String> {
    let jvms = cli::jvms_path().ok_or("未找到 jvms.exe")?;
    let args: Vec<&str> = if all { vec!["rls", "-a"] } else { vec!["rls"] };
    let out = cli::run(&jvms, &args, Duration::from_secs(60))?;
    if !out.success {
        return Err(format!(
            "{}。jvms 的远程索引位于 raw.githubusercontent.com，网络不通时需要为 jvms 配置代理",
            cli::failure_text(&out)
        ));
    }

    let installed: Vec<String> = load_status()
        .versions
        .into_iter()
        .map(|item| item.version)
        .collect();

    let mut items = Vec::new();
    for line in out.stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("use ")
            || trimmed.starts_with("For a complete list")
            || trimmed.starts_with("No availabled")
        {
            continue;
        }
        let Some(close) = trimmed.find(')') else { continue };
        let version = trimmed[close + 1..].trim().to_string();
        if version.is_empty() {
            continue;
        }
        let installed = installed.iter().any(|item| item == &version);
        items.push(RemoteVersion {
            version,
            group: None,
            installed,
        });
    }
    Ok(items)
}

#[tauri::command]
pub fn jdk_install(app: AppHandle, version: String) -> Result<String, String> {
    require_admin("安装 JDK")?;
    let jvms = cli::jvms_path().ok_or("未找到 jvms.exe")?;
    let task_id = format!("jdk-install-{}-{version}", now_millis());
    tasks::spawn(app, task_id.clone(), jvms, vec!["install".into(), version])?;
    Ok(task_id)
}

#[tauri::command]
pub fn jdk_remove(version: String) -> Result<CliOutput, String> {
    require_admin("卸载 JDK")?;
    let page = load_status();
    if page.status.current.as_deref() == Some(version.as_str()) {
        return Err("当前正在使用的版本不能卸载，请先切换到其他版本".into());
    }
    let jvms = cli::jvms_path().ok_or("未找到 jvms.exe")?;
    let out = cli::run(&jvms, &["remove", &version], Duration::from_secs(60))?;
    if !out.success {
        return Err(cli::failure_text(&out));
    }
    Ok(out)
}

#[tauri::command]
pub fn jdk_proxy() -> Result<String, String> {
    let jvms = cli::jvms_path().ok_or("未找到 jvms.exe")?;
    let out = cli::run(&jvms, &["proxy", "--show"], Duration::from_secs(10))?;
    Ok(if out.success {
        out.stdout.trim().to_string()
    } else {
        cli::failure_text(&out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jvms_list() {
        let text = "Installed jdk (* marks in use):\n    1) 23.0.1\n  * 2) 1.8.0_431\n";
        let items = parse_list(text);
        assert_eq!(items.len(), 2);
        assert!(!items[0].active);
        assert_eq!(items[0].version, "23.0.1");
        assert!(items[1].active);
        assert_eq!(items[1].version, "1.8.0_431");
    }

    #[test]
    fn parses_java_version_line() {
        let text = "java version \"1.8.0_431\"\nJava(TM) SE Runtime Environment";
        assert_eq!(parse_java_version(text).as_deref(), Some("1.8.0_431"));
    }

    #[test]
    fn compares_versions_with_prefix() {
        assert!(same_version("1.8.0_431", "1.8.0_431"));
        assert!(same_version("17.0.11", "17.0.11"));
        assert!(!same_version("17.0.11", "21.0.4"));
    }

    #[test]
    fn takes_last_link_segment() {
        assert_eq!(link_last_segment("C:\\jvms\\store\\17.0.11"), "17.0.11");
        assert_eq!(link_last_segment("C:/jvms/store/21.0.4"), "21.0.4");
    }
}
