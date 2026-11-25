use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::cli::{decode, CREATE_NO_WINDOW};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct Tasks(pub Mutex<HashMap<String, u32>>);

impl Tasks {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskEvent {
    task_id: String,
    kind: String,
    stream: String,
    line: Option<String>,
    code: Option<i32>,
    message: Option<String>,
}

pub fn spawn(
    app: AppHandle,
    task_id: String,
    program: PathBuf,
    args: Vec<String>,
) -> Result<(), String> {
    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 {} 失败：{e}", program.display()))?;

    let pid = child.id();
    if let Some(tasks) = app.try_state::<Tasks>() {
        tasks.0.lock().unwrap().insert(task_id.clone(), pid);
    }

    if let Some(pipe) = child.stdout.take() {
        let app = app.clone();
        let id = task_id.clone();
        std::thread::spawn(move || pump(app, id, "stdout", pipe));
    }
    if let Some(pipe) = child.stderr.take() {
        let app = app.clone();
        let id = task_id.clone();
        std::thread::spawn(move || pump(app, id, "stderr", pipe));
    }

    std::thread::spawn(move || {
        let status = child.wait();
        if let Some(tasks) = app.try_state::<Tasks>() {
            tasks.0.lock().unwrap().remove(&task_id);
        }
        let event = match status {
            Ok(s) if s.success() => TaskEvent {
                task_id,
                kind: "done".into(),
                stream: String::new(),
                line: None,
                code: s.code(),
                message: None,
            },
            Ok(s) => TaskEvent {
                task_id,
                kind: "error".into(),
                stream: String::new(),
                line: None,
                code: s.code(),
                message: Some("命令执行失败，详见日志".into()),
            },
            Err(e) => TaskEvent {
                task_id,
                kind: "error".into(),
                stream: String::new(),
                line: None,
                code: None,
                message: Some(e.to_string()),
            },
        };
        let _ = app.emit("task://event", event);
    });

    Ok(())
}

/// 逐字节切分 `\r` 与 `\n`，避免下载进度条用单回车刷新时看起来卡住。
fn pump(app: AppHandle, task_id: String, stream: &'static str, mut pipe: impl Read) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 2048];

    let flush = |buf: &mut Vec<u8>| {
        if buf.is_empty() {
            return;
        }
        let line = decode(buf);
        buf.clear();
        let _ = app.emit(
            "task://event",
            TaskEvent {
                task_id: task_id.clone(),
                kind: "line".into(),
                stream: stream.into(),
                line: Some(line),
                code: None,
                message: None,
            },
        );
    };

    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                for &b in &chunk[..n] {
                    if b == b'\n' || b == b'\r' {
                        flush(&mut buf);
                    } else {
                        buf.push(b);
                    }
                }
            }
            Err(_) => break,
        }
    }
    flush(&mut buf);
}
