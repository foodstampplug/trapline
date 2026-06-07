use crate::flags::{aggregate_findings, scan_line, Finding};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// One line of command output (mirrors Go OutLine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutLine {
    pub text: String,
    pub stream: String, // "out" | "err"
    pub spans: Vec<crate::flags::Span>,
}

/// Event payload emitted to the frontend via Tauri events
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QEvent {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    // line event fields
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spans: Option<Vec<crate::flags::Span>>,
    // done event fields
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    findings: Option<Vec<Finding>>,
}

/// Build a Child process for the given command line, using the configured shell.
/// On Windows, CREATE_NO_WINDOW prevents a console flash.
pub fn spawn_process(cmdline: &str, shell: &str) -> std::io::Result<Child> {
    let (prog, args) = shell_invocation(shell, cmdline);
    let mut cmd = Command::new(prog);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Suppress console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()
}

/// Returns (program, args) for the shell invocation.
/// Mirrors Go shellInvocation().
fn shell_invocation(shell: &str, cmdline: &str) -> (String, Vec<String>) {
    match shell {
        "cmd" => (
            "cmd".to_string(),
            vec!["/C".to_string(), cmdline.to_string()],
        ),
        "powershell" => (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                cmdline.to_string(),
            ],
        ),
        "bash" => (
            "bash".to_string(),
            vec!["-c".to_string(), cmdline.to_string()],
        ),
        "sh" => (
            "sh".to_string(),
            vec!["-c".to_string(), cmdline.to_string()],
        ),
        "" => {
            // Default: PowerShell on Windows, sh elsewhere
            #[cfg(target_os = "windows")]
            {
                (
                    "powershell".to_string(),
                    vec![
                        "-NoProfile".to_string(),
                        "-NonInteractive".to_string(),
                        "-Command".to_string(),
                        cmdline.to_string(),
                    ],
                )
            }
            #[cfg(not(target_os = "windows"))]
            {
                ("sh".to_string(), vec!["-c".to_string(), cmdline.to_string()])
            }
        }
        custom => (
            custom.to_string(),
            vec!["-c".to_string(), cmdline.to_string()],
        ),
    }
}

/// Run a command and stream output lines to the frontend as Tauri events.
/// The PID is registered in pids_map so cancel_command can kill it.
/// Mirrors Go runCommand().
pub fn run_command(
    app: AppHandle,
    id: String,
    cmdline: String,
    shell: String,
    pids_map: Arc<Mutex<std::collections::HashMap<String, u32>>>,
) {
    let start = Instant::now();
    let mut child = match spawn_process(&cmdline, &shell) {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit(
                "q_event",
                QEvent {
                    id: id.clone(),
                    kind: "done".to_string(),
                    text: None,
                    stream: None,
                    spans: None,
                    code: Some(-1),
                    ms: Some(0),
                    findings: Some(vec![]),
                },
            );
            eprintln!("Trapline runner: failed to spawn '{}': {}", cmdline, e);
            return;
        }
    };

    // Register PID
    let pid = child.id();
    {
        let mut map = pids_map.lock().unwrap();
        map.insert(id.clone(), pid);
    }

    let stdout = child.stdout.take().expect("stdout not captured");
    let stderr = child.stderr.take().expect("stderr not captured");

    // Collect all lines (stdout + stderr) with stream tag
    // We run them sequentially in two threads and collect into a shared vec.
    let lines_arc: Arc<Mutex<Vec<OutLine>>> = Arc::new(Mutex::new(Vec::new()));

    let lines_stdout = Arc::clone(&lines_arc);
    let app_stdout = app.clone();
    let id_stdout = id.clone();

    let stdout_handle = std::thread::spawn(move || {
        let reader = BufReader::with_capacity(4 * 1024 * 1024, stdout);
        for raw_line in reader.lines() {
            match raw_line {
                Ok(text) => {
                    let spans = scan_line(&text);
                    let line = OutLine {
                        text: text.clone(),
                        stream: "out".to_string(),
                        spans: spans.clone(),
                    };
                    {
                        let mut v = lines_stdout.lock().unwrap();
                        v.push(line);
                    }
                    let _ = app_stdout.emit(
                        "q_event",
                        QEvent {
                            id: id_stdout.clone(),
                            kind: "line".to_string(),
                            text: Some(text),
                            stream: Some("out".to_string()),
                            spans: Some(spans),
                            code: None,
                            ms: None,
                            findings: None,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });

    let lines_stderr = Arc::clone(&lines_arc);
    let app_stderr = app.clone();
    let id_stderr = id.clone();

    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::with_capacity(4 * 1024 * 1024, stderr);
        for raw_line in reader.lines() {
            match raw_line {
                Ok(text) => {
                    let spans = scan_line(&text);
                    let line = OutLine {
                        text: text.clone(),
                        stream: "err".to_string(),
                        spans: spans.clone(),
                    };
                    {
                        let mut v = lines_stderr.lock().unwrap();
                        v.push(line);
                    }
                    let _ = app_stderr.emit(
                        "q_event",
                        QEvent {
                            id: id_stderr.clone(),
                            kind: "line".to_string(),
                            text: Some(text),
                            stream: Some("err".to_string()),
                            spans: Some(spans),
                            code: None,
                            ms: None,
                            findings: None,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });

    let exit_status = child.wait();
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    let code = exit_status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let elapsed_ms = start.elapsed().as_millis() as u64;

    // De-register PID
    {
        let mut map = pids_map.lock().unwrap();
        map.remove(&id);
    }

    // Aggregate findings from all lines
    let all_lines = lines_arc.lock().unwrap();
    let findings = aggregate_findings(&all_lines);

    let _ = app.emit(
        "q_event",
        QEvent {
            id,
            kind: "done".to_string(),
            text: None,
            stream: None,
            spans: None,
            code: Some(code),
            ms: Some(elapsed_ms),
            findings: Some(findings),
        },
    );
}
