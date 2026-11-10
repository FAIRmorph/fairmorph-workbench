//! Project-scoped Workbench-to-Engine process transport and draft-file editor.
//! Scientific values and configuration are read by the Engine from project files.
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

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
        "run-pipeline" | "show-dataset-metadata" | "stage-dataset-metadata" | "approve-dataset-metadata" => {
            let directory = project.ok_or("RUN requires a project")?;
            cmd.arg(action).arg("--project").arg(directory);
        }
        _ => return Err("Unsupported Engine command".into()),
    }
    execute_command(app, cmd)
}

fn execute_command(app: AppHandle, mut cmd: Command) -> Result<Value, String> {
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

fn new_project_target(target: &str, name: &str) -> Result<PathBuf, String> {
    if name.trim().is_empty() {
        return Err("A project name is required".into());
    }
    let path = PathBuf::from(target);
    if !path.is_absolute() || path.exists() || path.is_symlink() {
        return Err("Choose a new absolute project path; existing folders are not overwritten".into());
    }
    let parent = path.parent().ok_or("Missing parent directory")?;
    if !parent.is_dir() || parent.is_symlink() {
        return Err("Parent directory must exist and not be a symlink".into());
    }
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let file_name = path.file_name().ok_or("Missing project directory name")?;
    Ok(parent.join(file_name))
}

fn import_source_path(source: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(source);
    if !path.is_absolute() || !path.is_file() || path.is_symlink() {
        return Err("Source must be an existing regular file at an absolute path".into());
    }
    path.canonicalize().map_err(|e| e.to_string())
}

fn resource_role(role: &str) -> Result<&str, String> {
    // These are technical import roles, not approved scientific entity types.
    if matches!(role, "events" | "counts") {
        Ok(role)
    } else {
        Err("Choose a resource role: events or counts".into())
    }
}

#[tauri::command]
pub async fn choose_project_parent(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = app.dialog().file().blocking_pick_folder();
        path.map(|item| item.into_path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string()))
            .transpose()
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn choose_source_files(app: AppHandle) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = app.dialog().file().blocking_pick_files().unwrap_or_default();
        paths.into_iter().map(|item| item.into_path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string())).collect()
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_project(
    app: AppHandle, project: String, name: String,
) -> Result<Value, String> {
    let target = new_project_target(&project, &name)?;
    let name = name.trim().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = engine_command(&app)?;
        cmd.arg("init-project").arg("--project").arg(&target)
            .arg("--name").arg(name);
        let result = execute_command(app, cmd)?;
        if result.get("project_id").and_then(Value::as_str).is_none() {
            return Err("Engine did not return a project identifier".into());
        }
        Ok(result)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_project_source(
    app: AppHandle, project: String, source: String, role: String,
) -> Result<Value, String> {
    let project = validate_project(&project)?;
    let source = import_source_path(&source)?;
    let role = resource_role(&role)?.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = engine_command(&app)?;
        cmd.arg("import-resource").arg("--project").arg(&project)
            .arg("--source").arg(&source).arg("--role").arg(&role);
        let result = execute_command(app, cmd)?;
        if result.get("resource_id").and_then(Value::as_str).is_none()
            || result.get("sha256").and_then(Value::as_str).is_none() {
            return Err("Engine did not return source identity and checksum".into());
        }
        Ok(result)
    }).await.map_err(|e| e.to_string())?
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


fn validate_revision_sha(sha: &str) -> Result<(), String> {
    if sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err("Invalid revision SHA-256".into());
    }
    Ok(())
}

fn draft_directory(project: &Path) -> Result<PathBuf, String> {
    let config = project.join("configurations");
    if !config.is_dir() || config.is_symlink() {
        return Err("Missing or unsafe configurations directory".into());
    }
    let dir = config.join("drafts");
    if dir.exists() && (!dir.is_dir() || dir.is_symlink()) {
        return Err("Unsafe drafts directory".into());
    }
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create draft directory: {e}"))?;
    Ok(dir)
}

fn atomic_draft_write(project: &Path, name: &str, document: &Value) -> Result<(), String> {
    let dir = draft_directory(project)?;
    let path = dir.join(name);
    if path.is_symlink() || (path.exists() && !path.is_file()) {
        return Err("Unsafe draft file".into());
    }
    let temporary = dir.join(format!(".{name}.{}.{}.tmp", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos()));
    let bytes = serde_json::to_vec_pretty(document).map_err(|e| e.to_string())?;
    let result = (|| {
        use std::io::Write;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temporary, &path).map_err(|e| e.to_string())
    })();
    let _ = fs::remove_file(&temporary);
    result
}

#[tauri::command]
pub async fn dataset_metadata_status(app: AppHandle, project: String) -> Result<Value, String> {
    let project = validate_project(&project)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute(app, "show-dataset-metadata", Some(&project))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_dataset_metadata_draft(
    project: String, title: String, parent_revision_sha256: Option<String>,
) -> Result<(), String> {
    let project = validate_project(&project)?;
    let title = title.trim();
    if title.chars().count() < 5 || title.chars().count() > 255 {
        return Err("Title must be between 5 and 255 characters".into());
    }
    if let Some(ref sha) = parent_revision_sha256 {
        validate_revision_sha(sha)?;
    }
    // A draft is NOT a revision or an approval. The Engine verifies the parent
    // against the current pointer on stage, before registering immutable bytes.
    let document = json!({
        "schema_version": "fairmorph.dataset-metadata/1",
        "title": title,
        "parent_revision_sha256": parent_revision_sha256,
    });
    atomic_draft_write(&project, "dataset-metadata.json", &document)
}

#[tauri::command]
pub async fn stage_dataset_metadata(app: AppHandle, project: String) -> Result<Value, String> {
    let project = validate_project(&project)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute(app, "stage-dataset-metadata", Some(&project))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn confirm_dataset_metadata(
    app: AppHandle, project: String, revision_sha256: String, confirmed_by: String,
) -> Result<Value, String> {
    let project = validate_project(&project)?;
    validate_revision_sha(&revision_sha256)?;
    let confirmed_by = confirmed_by.trim();
    if confirmed_by.is_empty() {
        return Err("A confirming user is required".into());
    }
    // The confirmation is intentionally a separate project-local decision file.
    // No semantic or review value is passed to Python via CLI arguments.
    atomic_draft_write(&project, "dataset-metadata-approval.json", &json!({
        "schema_version": "fairmorph.dataset-metadata-approval/1",
        "decision": "approve",
        "revision_sha256": revision_sha256,
        "confirmed_by": confirmed_by,
    }))?;
    tauri::async_runtime::spawn_blocking(move || {
        execute(app, "approve-dataset-metadata", Some(&project))
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{atomic_draft_write, validate_project, new_project_target, resource_role, import_source_path};
    use serde_json::json;

    #[test]
    fn validates_new_project_location_and_role() {
        let root = std::env::temp_dir();
        let target = root.join(format!("fairmorph-proposed-{}-unused", std::process::id()));
        assert_eq!(new_project_target(target.to_str().unwrap(), "Survey").unwrap(), target);
        assert!(new_project_target("", "Survey").is_err());
        assert!(new_project_target(root.to_str().unwrap(), "Survey").is_err());
        assert!(new_project_target(target.to_str().unwrap(), "").is_err());
        assert!(resource_role("events").is_ok());
        assert!(resource_role("counts").is_ok());
        assert!(resource_role("other").is_err());
        assert!(import_source_path(target.to_str().unwrap()).is_err());
    }

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
    #[test]
    fn only_draft_file_changes_when_editing_title() {
        let temporary = std::env::temp_dir().join(format!(
            "fairmorph-draft-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(temporary.join("configurations/pipelines")).unwrap();
        let project_json = temporary.join("project.json");
        std::fs::write(&project_json, "{}").unwrap();
        let pinned = temporary.join("configurations/pipelines/pinned.json");
        std::fs::write(&pinned, "unchanged").unwrap();
        let validated = validate_project(temporary.to_str().unwrap()).unwrap();

        atomic_draft_write(&validated, "dataset-metadata.json", &json!({
            "schema_version": "fairmorph.dataset-metadata/1",
            "title": "First scientific dataset title",
            "parent_revision_sha256": null
        })).unwrap();
        let draft = validated.join("configurations/drafts/dataset-metadata.json");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&draft).unwrap())
                .unwrap()["title"],
            "First scientific dataset title"
        );
        assert_eq!(std::fs::read_to_string(&pinned).unwrap(), "unchanged");

        atomic_draft_write(&validated, "dataset-metadata.json", &json!({
            "schema_version": "fairmorph.dataset-metadata/1",
            "title": "Second scientific dataset title",
            "parent_revision_sha256": null
        })).unwrap();
        assert!(std::fs::read_to_string(&draft).unwrap().contains("Second scientific"));
        assert_eq!(std::fs::read_to_string(&pinned).unwrap(), "unchanged");
        std::fs::remove_dir_all(&temporary).unwrap();
    }

}
