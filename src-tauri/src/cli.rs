use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::model::CliOutput;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn run(program: &PathBuf, args: &[&str], timeout: Duration) -> Result<CliOutput, String> {
    if !program.is_file() {
        return Err(format!("找不到可执行文件：{}", program.display()));
    }

    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 {} 失败：{e}", program.display()))?;

    let mut out_pipe = child.stdout.take().ok_or("无法读取标准输出")?;
    let mut err_pipe = child.stderr.take().ok_or("无法读取标准错误")?;

    let out_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let err_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_handle.join();
                    let _ = err_handle.join();
                    return Err(format!("命令执行超时（{} 秒）", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("等待进程失败：{e}"));
            }
        }
    };

    let stdout = decode(&out_handle.join().unwrap_or_default());
    let stderr = decode(&err_handle.join().unwrap_or_default());

    Ok(CliOutput {
        success: status.success(),
        code: status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

pub fn decode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (text, _, _) = encoding_rs::GBK.decode(bytes);
            text.to_string()
        }
    }
}

pub fn find_tool(exe: &str, fallbacks: &[String]) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(exe);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    for raw in fallbacks {
        let candidate = PathBuf::from(raw);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn jvms_path() -> Option<PathBuf> {
    find_tool("jvms.exe", &["C:\\jvms\\jvms.exe".to_string()])
}

pub fn nvm_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    find_tool("nvm.exe", &[format!("{appdata}\\nvm\\nvm.exe")])
}

pub fn is_admin() -> bool {
    #[cfg(windows)]
    unsafe {
        windows::Win32::UI::Shell::IsUserAnAdmin().as_bool()
    }
    #[cfg(not(windows))]
    false
}

pub fn read_link(path: &str) -> Option<String> {
    std::fs::read_link(path)
        .ok()
        .map(|target| target.to_string_lossy().to_string())
}

/// 把 CLI 的失败信息压成一行，优先用 stderr。
pub fn failure_text(out: &CliOutput) -> String {
    let raw = if out.stderr.trim().is_empty() {
        out.stdout.trim()
    } else {
        out.stderr.trim()
    };
    let line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("命令执行失败");
    format!("{line}（退出码 {}）", out.code)
}
