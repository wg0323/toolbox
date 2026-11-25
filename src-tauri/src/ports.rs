use std::collections::HashMap;
use std::ffi::c_void;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    TCP_TABLE_OWNER_PID_LISTENER,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

use crate::model::{KillResult, PortItem};

const PROTECTED: &[&str] = &[
    "system",
    "system idle process",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "smss.exe",
    "winlogon.exe",
    "svchost.exe",
    "dwm.exe",
];

struct RawPort {
    port: u16,
    address: String,
    pid: u32,
}

struct ProcInfo {
    name: String,
    exe: Option<String>,
    cmdline: Option<String>,
    start_time: Option<u64>,
}

fn collect_v4(out: &mut Vec<RawPort>) {
    unsafe {
        let mut size: u32 = 0;
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if size == 0 {
            return;
        }
        let mut buffer = vec![0u8; size as usize];
        let code = GetExtendedTcpTable(
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if code != 0 {
            return;
        }
        let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        for row in rows {
            out.push(RawPort {
                port: u16::from_be((row.dwLocalPort & 0xFFFF) as u16),
                address: Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string(),
                pid: row.dwOwningPid,
            });
        }
    }
}

fn collect_v6(out: &mut Vec<RawPort>) {
    unsafe {
        let mut size: u32 = 0;
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET6.0 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if size == 0 {
            return;
        }
        let mut buffer = vec![0u8; size as usize];
        let code = GetExtendedTcpTable(
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut size,
            false,
            AF_INET6.0 as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if code != 0 {
            return;
        }
        let table = &*(buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
        let rows = std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
        for row in rows {
            let addr = Ipv6Addr::from(row.ucLocalAddr);
            out.push(RawPort {
                port: u16::from_be((row.dwLocalPort & 0xFFFF) as u16),
                address: if addr.is_unspecified() {
                    "::".to_string()
                } else {
                    addr.to_string()
                },
                pid: row.dwOwningPid,
            });
        }
    }
}

fn snapshot_processes() -> HashMap<u32, ProcInfo> {
    let Ok(mut system) = System::new() else {
        return HashMap::new();
    };
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    let mut map = HashMap::new();
    for (pid, process) in system.processes() {
        let parts: Vec<String> = process
            .cmd()
            .iter()
            .map(|part| part.to_string_lossy().to_string())
            .collect();
        map.insert(
            pid.as_u32(),
            ProcInfo {
                name: process.name().to_string_lossy().to_string(),
                exe: process.exe().map(|path| path.to_string_lossy().to_string()),
                cmdline: if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(" "))
                },
                start_time: Some(process.start_time()),
            },
        );
    }
    map
}

fn classify(name: &str, cmdline: &str) -> &'static str {
    let lower_name = name.to_ascii_lowercase();
    let lower_cmd = cmdline.to_ascii_lowercase();

    if lower_name == "java.exe" || lower_name == "javaw.exe" {
        if lower_cmd.contains("jdt.ls")
            || lower_cmd.contains("lombok")
            || lower_cmd.contains("kotlin-daemon")
            || lower_cmd.contains("gradle-daemon")
            || lower_cmd.contains("maven")
        {
            return "java-ide";
        }
        if lower_cmd.contains("spring-boot")
            || lower_cmd.contains("spring_boot")
            || lower_cmd.contains("\\target\\classes")
            || lower_cmd.contains("/target/classes")
        {
            return "spring-boot";
        }
        return "java";
    }
    if lower_name == "node.exe" {
        return "node";
    }
    if lower_name.contains("docker") || lower_name == "vpnkit.exe" {
        return "docker";
    }
    "other"
}

fn is_protected(pid: u32, name: &str) -> bool {
    pid <= 4 || PROTECTED.contains(&name.to_ascii_lowercase().as_str())
}

pub fn list_ports() -> Vec<PortItem> {
    let mut raw = Vec::new();
    collect_v4(&mut raw);
    collect_v6(&mut raw);

    let processes = snapshot_processes();
    let mut items: Vec<PortItem> = raw
        .into_iter()
        .map(|entry| {
            let info = processes.get(&entry.pid);
            let name = info
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "未知进程".to_string());
            let cmdline = info.and_then(|item| item.cmdline.clone());
            let kind = classify(&name, cmdline.as_deref().unwrap_or(""));

            PortItem {
                port: entry.port,
                address: entry.address,
                pid: entry.pid,
                process_name: name.clone(),
                exe_path: info.and_then(|item| item.exe.clone()),
                cmdline,
                start_time: info.and_then(|item| item.start_time),
                kind: kind.to_string(),
                protected: is_protected(entry.pid, &name),
            }
        })
        .collect();

    items.sort_by_key(|item| (item.port, item.pid));
    merge_dual_stack(items)
}

/// 同一进程可能同时监听 IPv4 与 IPv6 通配地址，合并为一行，避免误判成两个程序。
fn merge_dual_stack(items: Vec<PortItem>) -> Vec<PortItem> {
    let mut merged: Vec<PortItem> = Vec::with_capacity(items.len());
    for item in items {
        if let Some(existing) = merged
            .iter_mut()
            .find(|entry| entry.pid == item.pid && entry.port == item.port)
        {
            if !existing.address.split(" / ").any(|part| part == item.address) {
                existing.address = format!("{} / {}", existing.address, item.address);
            }
            continue;
        }
        merged.push(item);
    }
    merged
}

fn terminate(pid: u32) -> Result<(), String> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|_| "无法打开进程：权限不足或进程已退出".to_string())?;
        let result = TerminateProcess(handle, 1).map_err(|_| "结束进程失败".to_string());
        let _ = CloseHandle(handle);
        result
    }
}

fn kill_and_wait(pid: u32) -> KillResult {
    let ports: Vec<u16> = list_ports()
        .into_iter()
        .filter(|item| item.pid == pid)
        .map(|item| item.port)
        .collect();

    if ports.is_empty() {
        return KillResult {
            success: false,
            message: "该进程已不再监听端口，可能已经退出".into(),
            released: true,
        };
    }

    if let Err(message) = terminate(pid) {
        return KillResult {
            success: false,
            message,
            released: false,
        };
    }

    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(400));
        let still_listening = list_ports()
            .iter()
            .any(|item| ports.contains(&item.port));
        if !still_listening {
            return KillResult {
                success: true,
                message: "进程已结束，端口已释放".into(),
                released: true,
            };
        }
    }

    KillResult {
        success: true,
        message: "进程已结束，但端口仍被占用（可能被其他进程接管）".into(),
        released: false,
    }
}

#[tauri::command]
pub fn ports_list() -> Vec<PortItem> {
    list_ports()
}

#[tauri::command]
pub fn port_kill(pid: u32) -> KillResult {
    let items = list_ports();
    let Some(item) = items.iter().find(|entry| entry.pid == pid) else {
        return KillResult {
            success: false,
            message: "进程已退出".into(),
            released: true,
        };
    };

    if item.protected {
        return KillResult {
            success: false,
            message: format!("{} 属于系统关键进程，已阻止结束", item.process_name),
            released: false,
        };
    }

    kill_and_wait(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_listening_ports() {
        let items = list_ports();
        assert!(!items.is_empty(), "本机应至少存在一个 TCP 监听端口");
        for item in &items {
            assert!(item.port > 0);
            assert!(!item.process_name.is_empty());
        }
    }

    #[test]
    fn classifies_process_kind() {
        assert_eq!(
            classify("java.exe", "java -jar app.jar --spring-boot"),
            "spring-boot"
        );
        assert_eq!(
            classify("java.exe", "java -cp D:\\proj\\target\\classes com.demo.App"),
            "spring-boot"
        );
        assert_eq!(
            classify("java.exe", "java ... org.eclipse.jdt.ls ..."),
            "java-ide"
        );
        assert_eq!(classify("node.exe", "node server.js"), "node");
        assert_eq!(classify("com.docker.backend.exe", ""), "docker");
    }

    #[test]
    fn protects_system_processes() {
        assert!(is_protected(4, "System"));
        assert!(is_protected(900, "lsass.exe"));
        assert!(!is_protected(9000, "java.exe"));
    }

    #[test]
    fn merges_dual_stack_listeners() {
        let sample = |address: &str, pid: u32| PortItem {
            port: 8080,
            address: address.to_string(),
            pid,
            process_name: "java.exe".into(),
            exe_path: None,
            cmdline: None,
            start_time: None,
            kind: "java".into(),
            protected: false,
        };

        let merged = merge_dual_stack(vec![sample("0.0.0.0", 25292), sample("::", 25292)]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].address, "0.0.0.0 / ::");

        let merged = merge_dual_stack(vec![sample("0.0.0.0", 25292), sample("0.0.0.0", 1)]);
        assert_eq!(merged.len(), 2);
    }
}
