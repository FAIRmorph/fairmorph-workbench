use std::process::Command;

#[test]
fn test_engine_cli_poc_execution() {
    #[cfg(target_os = "windows")]
    let py_bin = "../../fairmorph-engine/.venv/Scripts/python.exe";
    #[cfg(not(target_os = "windows"))]
    let py_bin = "../../fairmorph-engine/.venv/bin/python";

    // Starte den Engine-Subprozess direkt aus dem Rust-Testrunner
    let output = Command::new(py_bin)
        .args(["-m", "fairmorph_engine.cli", "poc", "--records", "20"])
        .output()
        .expect("Subprozess konnte nicht gestartet werden");

    assert!(output.status.success(), "Engine sollte mit Exit-Code 0 beenden");

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Prüfe direkt in Rust auf NDJSON-Integrität
    assert!(stdout.contains("\"event\": \"progress\""));
    assert!(stdout.contains("\"event\": \"result\""));
    assert!(stdout.contains("\"status\": \"success\""));
}