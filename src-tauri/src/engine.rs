use tauri::{AppHandle, Emitter};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

#[cfg(debug_assertions)]
use std::path::PathBuf;

#[cfg(not(debug_assertions))]
use tauri::Manager;


#[cfg(debug_assertions)]
fn resolve_python_bin() -> Result<PathBuf, String> {
    // 1. Optional: Überprüfe eine explizite Umgebungsvariable
    if let Ok(env_path) = std::env::var("FAIRMORPH_ENGINE_PYTHON") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    // 2. Suche relative Pfade abhängig vom aktuellen Arbeitsverzeichnis
    #[cfg(target_os = "windows")]
    let candidates = [
        "../../fairmorph-engine/.venv/Scripts/python.exe", // wenn cwd = src-tauri
        "../fairmorph-engine/.venv/Scripts/python.exe",    // wenn cwd = repo root
    ];

    #[cfg(not(target_os = "windows"))]
    let candidates = [
        "../../fairmorph-engine/.venv/bin/python", // wenn cwd = src-tauri
        "../fairmorph-engine/.venv/bin/python",    // wenn cwd = repo root
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    Err(format!(
        "Python executable not found. Tried paths: {:?} (Current working directory: {:?})",
        candidates, cwd
    ))
}

#[tauri::command]
pub async fn run_poc(app: AppHandle, records: u32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        #[cfg(debug_assertions)]
        let mut cmd = {
            let python = resolve_python_bin()?;
            let mut c = Command::new(python);
            c.args(["-m", "fairmorph_engine.cli"]);
            c
        };

        #[cfg(not(debug_assertions))]
        let mut cmd = {
            let resource_dir = app
                .path()
                .resource_dir()
                .map_err(|e| format!("Resource dir error: {}", e))?;

            #[cfg(target_os = "windows")]
            let bin_name = "fairmorph-engine.exe";
            #[cfg(not(target_os = "windows"))]
            let bin_name = "fairmorph-engine";

            let bin_path = resource_dir.join("engine").join(bin_name);
            if !bin_path.exists() {
                return Err(format!("Engine binary not found at {:?}", bin_path));
            }
            Command::new(bin_path)
        };

        cmd.args(["poc", "--records", &records.to_string()]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start engine process: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        // Drain stderr concurrently to avoid blocking when the pipe fills.
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let stderr_reader = std::thread::spawn(move || {
            let mut message = String::new();
            let mut stderr = stderr;
            let _ = stderr.read_to_string(&mut message);
            message
        });
        let reader = BufReader::new(stdout);

        let mut stream_error = None;
        for line in reader.lines() {
            match line {
                Ok(line) if !line.trim().is_empty() => {
                    if let Err(error) = app.emit("poc-event", line) {
                        stream_error = Some(format!("Failed to emit engine event: {error}"));
                        break;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    stream_error = Some(format!("Failed to read engine output: {error}"));
                    break;
                }
            }
        }

        if stream_error.is_some() {
            let _ = child.kill();
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let err_msg = stderr_reader
            .join()
            .map_err(|_| "Failed to read engine stderr".to_string())?;
        if let Some(error) = stream_error {
            return Err(error);
        }
        if !status.success() {
            return Err(if err_msg.is_empty() {
                format!("Engine exited with status {}", status)
            } else {
                format!("Engine exited with status {}: {}", status, err_msg)
            });
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

