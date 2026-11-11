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
        "run-pipeline" | "verify-project" | "show-dataset-metadata" | "stage-dataset-metadata" | "approve-dataset-metadata" => {
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

/// Select an already initialized FAIRmorph project, without creating or changing files.
#[tauri::command]
pub async fn choose_existing_project(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app.dialog().file().blocking_pick_folder();
        match picked {
            None => Ok(None),
            Some(folder) => {
                let path = folder.into_path().map_err(|e| e.to_string())?;
                let canonical = validate_project(&path.to_string_lossy())?;
                Ok(Some(canonical.to_string_lossy().into_owned()))
            }
        }
    }).await.map_err(|e| e.to_string())?
}

/// Read Engine-created import manifests. Resource IDs follow the Engine's
/// immutable import identity: resource-{role}-{source_sha256}.
fn project_imported_resources(project: &Path) -> Result<Value, String> {
    let imports = project.join("imports");
    if !imports.is_dir() || imports.is_symlink() {
        return Err("Project has no regular imports directory".into());
    }
    let mut resources = Vec::new();
    for item in fs::read_dir(&imports).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let directory = item.path();
        if item.file_type().map_err(|e| e.to_string())?.is_symlink() || !directory.is_dir() {
            return Err("Unsafe import record directory".into());
        }
        let manifest_path = directory.join("manifest.json");
        if manifest_path.is_symlink() || !manifest_path.is_file() {
            return Err("Missing regular import manifest".into());
        }
        let manifest: Value = serde_json::from_slice(
            &fs::read(&manifest_path).map_err(|e| e.to_string())?
        ).map_err(|e| format!("Invalid import manifest: {e}"))?;
        if manifest.get("schema_version").and_then(Value::as_str) != Some("fairmorph.poc.import/1") {
            return Err("Unsupported import manifest schema".into());
        }
        let id = manifest.get("import_id").and_then(Value::as_str)
            .ok_or("Missing import identity")?;
        let sha = manifest.get("sha256").and_then(Value::as_str)
            .ok_or("Missing source checksum")?;
        let role = manifest.get("role").and_then(Value::as_str)
            .ok_or("Missing import role")?;
        let name = manifest.get("original_name").and_then(Value::as_str)
            .ok_or("Missing original filename")?;
        if !role.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
            || role.is_empty() || id != format!("{role}-{sha}")
            || sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            || name.is_empty() || item.file_name().to_string_lossy() != id
            || manifest.get("object_path").and_then(Value::as_str)
                != Some(format!("objects/sha256/{}/{}", &sha[..2], sha).as_str()) {
            return Err("Inconsistent import manifest identity".into());
        }
        let object = project.join("objects/sha256").join(&sha[..2]).join(sha);
        if object.is_symlink() || !object.is_file() {
            return Err("Imported source object missing or unsafe".into());
        }
        resources.push(json!({
            "resource_id": format!("resource-{id}"),
            "original_name": name,
            "role": role,
            "sha256": sha
        }));
    }
    resources.sort_by(|a, b| a["resource_id"].as_str().cmp(&b["resource_id"].as_str()));
    Ok(json!({"resources": resources}))
}

#[tauri::command]
pub async fn list_project_resources(project: String) -> Result<Value, String> {
    let project = validate_project(&project)?;
    tauri::async_runtime::spawn_blocking(move || project_imported_resources(&project))
        .await.map_err(|e| e.to_string())?
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


#[derive(serde::Deserialize)]
pub struct PipelineBinding {
    slot: String,
    resource_id: String,
    format: String,
}

fn technical_pipeline_template(template: &Path) -> Result<Value, String> {
    if !template.is_absolute() || !template.is_file() || template.is_symlink() {
        return Err("Pipeline template must be an existing regular JSON file".into());
    }
    let data: Value = serde_json::from_slice(
        &fs::read(template).map_err(|e| format!("Cannot read pipeline template: {e}"))?
    ).map_err(|e| format!("Invalid pipeline JSON: {e}"))?;
    let obj = data.as_object().ok_or("Pipeline must be a JSON object")?;
    if obj.keys().any(|key| !matches!(key.as_str(), "schema_version" | "resources" | "steps" | "outputs")) {
        return Err("MVP accepts technical DAG templates only, without entity/profile metadata".into());
    }
    if data.get("schema_version").and_then(Value::as_str) != Some("fairmorph.pipeline/1") {
        return Err("Unsupported pipeline schema_version".into());
    }
    let steps = data.get("steps").and_then(Value::as_array)
        .filter(|a| !a.is_empty()).ok_or("Template requires a nonempty steps array")?;
    let outputs = data.get("outputs").and_then(Value::as_array)
        .filter(|a| !a.is_empty()).ok_or("Template requires a nonempty outputs array")?;
    if !steps.iter().all(Value::is_object) || !outputs.iter().all(Value::is_string) {
        return Err("Invalid technical pipeline steps or outputs".into());
    }
    let resources = data.get("resources").and_then(Value::as_object)
        .filter(|m| !m.is_empty()).ok_or("Template requires declared resource slots")?;
    for (slot, binding) in resources {
        if slot.is_empty() || !slot.bytes().next().unwrap_or(b' ').is_ascii_alphabetic()
            || !slot.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-') {
            return Err("Invalid template resource slot".into());
        }
        if !binding.is_object() {
            return Err("Invalid template resource binding".into());
        }
        if !matches!(binding.get("format").and_then(Value::as_str), Some("csv" | "parquet")) {
            return Err("Template resource format must be csv or parquet".into());
        }
    }
    Ok(data)
}

fn configured_pipeline(template: &Path, bindings: &[PipelineBinding]) -> Result<Value, String> {
    let mut config = technical_pipeline_template(template)?;
    let resources = config.get_mut("resources").and_then(Value::as_object_mut)
        .ok_or("Template resources missing")?;
    if bindings.len() != resources.len() {
        return Err("Every declared pipeline input must be bound exactly once".into());
    }
    let mut seen = std::collections::HashSet::new();
    for binding in bindings {
        if !seen.insert(binding.slot.as_str()) {
            return Err("Duplicate pipeline input binding".into());
        }
        if binding.resource_id.trim().is_empty()
            || !matches!(binding.format.as_str(), "csv" | "parquet") {
            return Err("Invalid resource identity or input format".into());
        }
        let input = resources.get_mut(&binding.slot)
            .ok_or("Binding refers to an unknown template resource slot")?;
        *input = json!({"resource_id": binding.resource_id, "format": binding.format});
    }
    Ok(config)
}

#[tauri::command]
pub async fn choose_pipeline_template(app: AppHandle) -> Result<Option<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app.dialog().file().add_filter("Pipeline JSON", &["json"])
            .blocking_pick_file();
        match picked {
            None => Ok(None),
            Some(file) => {
                let path = file.into_path().map_err(|e| e.to_string())?;
                let config = technical_pipeline_template(&path)?;
                let inputs: Vec<Value> = config["resources"].as_object().ok_or("No resources")?
                    .iter().map(|(name, binding)| json!({
                        "name": name, "format": binding["format"]
                    })).collect();
                Ok(Some(json!({
                    "path": path.to_string_lossy(),
                    "inputs": inputs,
                    "outputs": config["outputs"],
                    "steps": config["steps"].as_array().map(Vec::len).unwrap_or(0)
                })))
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_project_pipeline_draft(
    project: String, template: String, bindings: Vec<PipelineBinding>,
) -> Result<Value, String> {
    let project = validate_project(&project)?;
    let template = PathBuf::from(&template);
    let config = configured_pipeline(&template, &bindings)?;
    atomic_draft_write(&project, "pipeline-technical.json", &config)?;
    Ok(json!({"path": project.join("configurations/drafts/pipeline-technical.json")
        .to_string_lossy(), "inputs": bindings.len()}))
}

#[tauri::command]
pub async fn register_project_pipeline(
    app: AppHandle, project: String,
) -> Result<Value, String> {
    let project = validate_project(&project)?;
    let definition = project.join("configurations/drafts/pipeline-technical.json");
    if !definition.is_file() || definition.is_symlink() {
        return Err("Save the pipeline draft before registration".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = engine_command(&app)?;
        cmd.arg("register-pipeline").arg("--project").arg(&project)
            .arg("--definition").arg(&definition);
        let result = execute_command(app, cmd)?;
        if result.get("pipeline_sha256").and_then(Value::as_str).is_none() {
            return Err("Engine returned no pipeline SHA-256".into());
        }
        Ok(result)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn select_registered_pipeline(
    app: AppHandle, project: String, pipeline_sha256: String,
) -> Result<Value, String> {
    let project = validate_project(&project)?;
    validate_revision_sha(&pipeline_sha256)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = engine_command(&app)?;
        cmd.arg("select-pipeline").arg("--project").arg(&project)
            .arg("--pipeline-sha256").arg(&pipeline_sha256);
        let result = execute_command(app, cmd)?;
        if result.get("active_pipeline_sha256").and_then(Value::as_str)
            != Some(pipeline_sha256.as_str()) {
            return Err("Engine did not select the requested pipeline revision".into());
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
pub async fn verify_project(app: AppHandle, project: String) -> Result<Value, String> {
    let project = validate_project(&project)?;
    tauri::async_runtime::spawn_blocking(move || execute(app, "verify-project", Some(&project)))
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
    use super::{atomic_draft_write, validate_project, new_project_target, resource_role, import_source_path, configured_pipeline, PipelineBinding};
    use serde_json::json;

    #[test]
    fn validates_new_project_location_and_role() {
        let root = std::env::temp_dir();
        let target = root.join(format!("fairmorph-proposed-{}-unused", std::process::id()));
        let expected = root.canonicalize().unwrap().join(target.file_name().unwrap());
        assert_eq!(new_project_target(target.to_str().unwrap(), "Survey").unwrap(), expected);
        assert!(new_project_target("", "Survey").is_err());
        assert!(new_project_target(root.to_str().unwrap(), "Survey").is_err());
        assert!(new_project_target(target.to_str().unwrap(), "").is_err());
        assert!(resource_role("events").is_ok());
        assert!(resource_role("counts").is_ok());
        assert!(resource_role("other").is_err());
        assert!(import_source_path(target.to_str().unwrap()).is_err());
    }


    #[test]
    fn pipeline_binding_requires_all_explicit_slots_and_preserves_steps() {
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-template-test-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("template.json");
        let source = json!({
            "schema_version": "fairmorph.pipeline/1",
            "resources": {"survey": {"resource_id": "UNBOUND", "format": "csv"}},
            "steps": [{
                "id": "keep", "operator": "table.select/1",
                "inputs": ["survey"], "output": "selected",
                "parameters": {"columns": ["id"]}
            }],
            "outputs": ["selected"]
        });
        std::fs::write(&path, serde_json::to_vec(&source).unwrap()).unwrap();
        let bindings = vec![PipelineBinding {
            slot: "survey".into(), resource_id: "resource-a".into(), format: "parquet".into()
        }];
        let compiled = configured_pipeline(&path, &bindings).unwrap();
        assert_eq!(compiled["steps"], source["steps"]);
        assert_eq!(compiled["resources"]["survey"]["resource_id"], "resource-a");
        assert_eq!(compiled["resources"]["survey"]["format"], "parquet");
        assert!(configured_pipeline(&path, &[]).is_err());
        assert!(configured_pipeline(&path, &[PipelineBinding {
            slot: "unknown".into(), resource_id: "resource-a".into(), format: "csv".into()
        }]).is_err());
        let mut scientific = source;
        scientific["entities"] = json!({});
        std::fs::write(&path, serde_json::to_vec(&scientific).unwrap()).unwrap();
        assert!(configured_pipeline(&path, &bindings).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }


    #[test]
    fn invalid_pipeline_templates_are_rejected_without_writing_anything() {
        use super::technical_pipeline_template;
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-invalid-template-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("template.json");
        for invalid in [
            "not json",
            "[]",
            r#"{"schema_version":"other","resources":{"events":{"format":"csv"}},"steps":[{}],"outputs":["out"]}"#,
            r#"{"schema_version":"fairmorph.pipeline/1","resources":{},"steps":[{}],"outputs":["out"]}"#,
            r#"{"schema_version":"fairmorph.pipeline/1","resources":{"events":{"format":"json"}},"steps":[{}],"outputs":["out"]}"#,
            r#"{"schema_version":"fairmorph.pipeline/1","resources":{"events":{"format":"csv"}},"steps":[],"outputs":["out"]}"#,
            r#"{"schema_version":"fairmorph.pipeline/1","resources":{"events":{"format":"csv"}},"steps":[{}],"outputs":["out"],"entities":{}}"#
        ] {
            std::fs::write(&file, invalid).unwrap();
            assert!(technical_pipeline_template(&file).is_err(), "accepted {invalid}");
        }
        assert!(technical_pipeline_template(&dir.join("missing.json")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn duplicate_and_incomplete_pipeline_bindings_are_rejected() {
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-bindings-test-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let template = dir.join("pipeline.json");
        std::fs::write(&template, serde_json::to_vec(&json!({
            "schema_version": "fairmorph.pipeline/1",
            "resources": {"events": {"format": "csv"}, "counts": {"format": "csv"}},
            "steps": [{"id":"combine","operator":"table.join/1",
                "inputs":["events","counts"],"output":"combined","parameters":{"keys":["id"],"cardinality":"m:1"}}],
            "outputs": ["combined"]
        })).unwrap()).unwrap();
        let bind = |slot: &str, format: &str| PipelineBinding {
            slot: slot.into(), resource_id: format!("resource-{slot}"), format: format.into()
        };
        assert!(configured_pipeline(&template, &[]).is_err());
        assert!(configured_pipeline(&template, &[bind("events","csv")]).is_err());
        assert!(configured_pipeline(&template,
            &[bind("events","csv"), bind("events","csv")]).is_err());
        assert!(configured_pipeline(&template,
            &[bind("events","csv"), bind("missing","csv")]).is_err());
        assert!(configured_pipeline(&template,
            &[bind("events","csv"), bind("counts","json")]).is_err());
        let valid = configured_pipeline(&template,
            &[bind("events","csv"), bind("counts","parquet")]).unwrap();
        assert_eq!(valid["resources"]["events"]["resource_id"], "resource-events");
        assert_eq!(valid["resources"]["counts"]["format"], "parquet");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lists_immutable_existing_imports_without_writing_project() {
        use super::project_imported_resources;
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-import-reader-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        let sha = "a".repeat(64);
        let id = format!("counts-{sha}");
        let imports = dir.join("imports").join(&id);
        std::fs::create_dir_all(&imports).unwrap();
        let object = dir.join("objects/sha256/aa").join(&sha);
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        std::fs::write(&object, b"species,total_count\\nSeal,0\\n").unwrap();
        let manifest = json!({
            "schema_version": "fairmorph.poc.import/1",
            "import_id": id,
            "role": "counts",
            "original_name": "counts.csv",
            "sha256": sha,
            "object_path": format!("objects/sha256/aa/{sha}")
        });
        let path = imports.join("manifest.json");
        let bytes = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let result = project_imported_resources(&dir).unwrap();
        assert_eq!(result["resources"][0]["resource_id"], format!("resource-{id}"));
        assert_eq!(result["resources"][0]["original_name"], "counts.csv");
        assert_eq!(result["resources"][0]["role"], "counts");
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::write(&path, b"{}").unwrap();
        assert!(project_imported_resources(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn existing_project_folder_requires_regular_project_descriptor() {
        use super::validate_project;
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-project-picker-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        assert!(validate_project(dir.to_str().unwrap()).is_err());
        let descriptor = dir.join("project.json");
        std::fs::write(&descriptor, b"{}").unwrap();
        assert_eq!(
            validate_project(dir.to_str().unwrap()).unwrap(),
            dir.canonicalize().unwrap()
        );
        std::fs::remove_file(&descriptor).unwrap();
        std::fs::create_dir(&descriptor).unwrap();
        assert!(validate_project(dir.to_str().unwrap()).is_err());
        std::fs::remove_dir(&descriptor).unwrap();
        #[cfg(unix)]
        {
            let other = dir.join("other.json");
            std::fs::write(&other, b"{}").unwrap();
            std::os::unix::fs::symlink(&other, &descriptor).unwrap();
            assert!(validate_project(dir.to_str().unwrap()).is_err());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_symlink_source_and_does_not_modify_existing_project() {
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-source-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let source = dir.join("raw.csv");
        std::fs::write(&source, b"id,count\\n1,0\\n").unwrap();
        let saved = std::fs::read(&source).unwrap();
        assert_eq!(import_source_path(source.to_str().unwrap()).unwrap(), source.canonicalize().unwrap());
        assert!(new_project_target(dir.to_str().unwrap(), "Existing").is_err());
        assert_eq!(std::fs::read(&source).unwrap(), saved);
        #[cfg(unix)]
        {
            let link = dir.join("source-link.csv");
            std::os::unix::fs::symlink(&source, &link).unwrap();
            assert!(import_source_path(link.to_str().unwrap()).is_err());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pipeline_draft_write_preserves_unrelated_immutable_configuration() {
        let dir = std::env::temp_dir().join(format!(
            "fairmorph-draft-safety-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap().as_nanos()
        ));
        std::fs::create_dir_all(dir.join("configurations/pipelines")).unwrap();
        let pinned = dir.join("configurations/pipelines/immutable.json");
        std::fs::write(&pinned, "pinned").unwrap();
        let draft = json!({"schema_version": "fairmorph.pipeline/1", "resources": {},
            "steps": [], "outputs": []});
        atomic_draft_write(&dir, "pipeline-technical.json", &draft).unwrap();
        assert_eq!(std::fs::read_to_string(&pinned).unwrap(), "pinned");
        assert_eq!(serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(dir.join("configurations/drafts/pipeline-technical.json")).unwrap()
        ).unwrap(), draft);
        std::fs::remove_dir_all(dir).unwrap();
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
