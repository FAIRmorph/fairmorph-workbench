use tauri::{AppHandle, Emitter};
use std::io::{BufRead, BufReader, Read};
use tauri_plugin_dialog::DialogExt;
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

fn execute_engine_command(
    app: AppHandle,
    arguments: &[String],
    event_name: &'static str,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    let mut cmd = {
        let python = resolve_python_bin()?;
        let mut command = Command::new(python);
        command.args(["-m", "fairmorph_engine.cli"]);
        command
    };

    #[cfg(not(debug_assertions))]
    let mut cmd = {
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|error| format!("Resource dir error: {error}"))?;

        #[cfg(target_os = "windows")]
        let bin_name = "fairmorph-engine.exe";
        #[cfg(not(target_os = "windows"))]
        let bin_name = "fairmorph-engine";

        let binary = resource_dir.join("engine").join(bin_name);
        if !binary.is_file() {
            return Err(format!("Engine binary not found at {:?}", binary));
        }
        Command::new(binary)
    };

    cmd.args(arguments);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let stderr_reader = std::thread::spawn(move || {
        let mut message = String::new();
        let _ = stderr.read_to_string(&mut message);
        message
    });

    for line in BufReader::new(stdout).lines() {
        if let Ok(line) = line {
            if !line.trim().is_empty() {
                let _ = app.emit(event_name, line);
            }
        }
    }

    let status = child.wait().map_err(|error| error.to_string())?;
    let stderr_output = stderr_reader
        .join()
        .map_err(|_| "Failed to read engine stderr".to_string())?;
    if !status.success() {
        return Err(if stderr_output.is_empty() {
            format!("Engine exited with status {status}")
        } else {
            format!("Engine exited with status {status}: {stderr_output}")
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn run_poc(app: AppHandle, records: u32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_engine_command(app, &["poc".into(), "--records".into(), records.to_string()], "poc-event")
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn select_csv_file(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("CSV files", &["csv"])
            .blocking_pick_file()
            .map(|selected| {
                selected
                    .into_path()
                    .map(|path| path.to_string_lossy().into_owned())
                    .map_err(|error| error.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn inspect_csv(app: AppHandle, source: String) -> Result<(), String> {
    if source.trim().is_empty() {
        return Err("Select a CSV file first".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        execute_engine_command(app, &["inspect-csv".into(), source], "csv-event")
    })
    .await
    .map_err(|error| error.to_string())?
}
