//! Minimal project-scoped, read-only Workbench-to-Engine process transport.
//! Scientific values and configuration are read by the Engine from project files.
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[cfg(not(debug_assertions))]
use tauri::Manager;

#[cfg(debug_assertions)]
fn resolve_python_bin() -> Result<PathBuf, String> {
    if let Ok(candidate) = std::env::var("FAIRMORPH_ENGINE_PYTHON") {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    #[cfg(target_os = "windows")]
    let candidates = [
        "../../fairmorph-engine/.venv/Scripts/python.exe",
        "../fairmorph-engine/.venv/Scripts/python.exe",
    ];
    #[cfg(not(target_os = "windows"))]
    let candidates = [
        "../../fairmorph-engine/.venv/bin/python",
        "../fairmorph-engine/.venv/bin/python",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("Engine Python not found; set FAIRMORPH_ENGINE_PYTHON".into())
}

fn validate_project(project: &str) -> Result<PathBuf, String> {
    if project.trim().is_empty() {
        return Err("A project directory is required".into());
    }
    let path = PathBuf::from(project);
    if path.is_symlink() || !path.is_dir() {
        return Err("Project must be an existing non-symlink directory".into());
    }
    let descriptor = path.join("project.json");
    if descriptor.is_symlink() || !descriptor.is_file() {
        return Err("The selected directory has no regular project.json".into());
    }
    path.canonicalize().map_err(|e| format!("Project path error: {e}"))
}

fn engine_command(app: &AppHandle) -> Result<Command, String> {
    #[cfg(debug_assertions)]
    {
        let mut command = Command::new(resolve_python_bin()?);
        command.args(["-m", "fairmorph_engine.cli"]);
        let _ = app;
        Ok(command)
    }
    #[cfg(not(debug_assertions))]
    {
        let dir = app.path().resource_dir().map_err(|e| e.to_string())?;
        #[cfg(target_os = "windows")]
        let binary = dir.join("engine/fairmorph-engine.exe");
        #[cfg(not(target_os = "windows"))]
        let binary = dir.join("engine/fairmorph-engine");
        if !binary.is_file() {
            return Err(format!("Engine executable missing: {}", binary.display()));
        }
        Ok(Command::new(binary))
    }
}

fn execute(app: AppHandle, action: &str, project: Option<&Path>) -> Result<Value, String> {
    let mut cmd = engine_command(&app)?;
    match action {
        "handshake" => { cmd.arg("handshake"); }
        "run-pipeline" => {
            let directory = project.ok_or("RUN requires a project")?;
            cmd.arg("run-pipeline").arg("--project").arg(directory);
        }
        _ => return Err("Unsupported Engine command".into()),
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Cannot start Engine: {e}"))?;
    let stdout = child.stdout.take().ok_or("Missing Engine stdout")?;
    let mut stderr = child.stderr.take().ok_or("Missing Engine stderr")?;
    let stderr_reader = std::thread::spawn(move || {
        let mut message = String::new();
        let _ = stderr.read_to_string(&mut message);
        message
    });
    let mut final_result: Option<Value> = None;
    let mut protocol_error: Option<String> = None;
    for line in BufReader::new(stdout).lines() {
        match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => {
                match serde_json::from_str::<Value>(&line) {
                    Ok(message) => {
                        match message.get("event").and_then(Value::as_str) {
                            Some("progress") => {
                                if let Err(error) = app.emit("engine-progress", &message) {
                                    protocol_error = Some(format!("Cannot display progress: {error}"));
                                    break;
                                }
                            }
                            Some("result") => {
                                if final_result.is_some() {
                                    protocol_error = Some("Multiple Engine results".into());
                                    break;
                                }
                                final_result = Some(message);
                            }
                            _ => {
                                protocol_error = Some("Unexpected Engine event".into());
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        protocol_error = Some(format!("Invalid Engine NDJSON: {error}"));
                        break;
                    }
                }
            }
            Err(error) => {
                protocol_error = Some(format!("Cannot read Engine stdout: {error}"));
                break;
            }
        }
    }
    if protocol_error.is_some() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostic = stderr_reader.join().map_err(|_| "stderr reader failed")?;
    if let Some(error) = protocol_error {
        return Err(error);
    }
    let result = final_result.ok_or_else(|| format!("Engine returned no result: {diagnostic}"))?;
    if !status.success() || result.get("status").and_then(Value::as_str) != Some("success") {
        let message = result.get("message").and_then(Value::as_str).unwrap_or("Engine failed");
        return Err(format!("{message}{}", if diagnostic.is_empty() { String::new() } else { format!(": {diagnostic}") }));
    }
    result.get("data").cloned().ok_or("Engine success has no data".into())
}

#[tauri::command]
pub async fn engine_handshake(app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || execute(app, "handshake", None))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_project(app: AppHandle, project: String) -> Result<Value, String> {
    let project = validate_project(&project)?;
    tauri::async_runtime::spawn_blocking(move || execute(app, "run-pipeline", Some(&project)))
        .await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::validate_project;

    #[test]
    fn rejects_missing_project() {
        assert!(validate_project("").is_err());
        assert!(validate_project("/not/a/real/fairmorph/project").is_err());
    }

    #[test]
    fn requires_project_descriptor() {
        let temporary = std::env::temp_dir().join(format!("fairmorph-workbench-{}", std::process::id()));
        std::fs::create_dir_all(&temporary).unwrap();
        assert!(validate_project(temporary.to_str().unwrap()).is_err());
        std::fs::remove_dir_all(&temporary).unwrap();
    }
}
