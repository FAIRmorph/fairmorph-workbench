use std::process::Command;

#[test]
fn test_engine_cli_poc_execution() {
    let python = std::env::var("FAIRMORPH_ENGINE_PYTHON")
        .expect("Set FAIRMORPH_ENGINE_PYTHON to the Python interpreter with fairmorph-engine installed");
    let output = Command::new(python)
        .args(["-m", "fairmorph_engine.cli", "poc", "--records", "20"])
        .output()
        .expect("Could not launch engine subprocess");

    assert!(
        output.status.success(),
        "Engine exited with {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("Invalid NDJSON line"))
        .collect();
    assert!(events.iter().any(|event| event["event"] == "progress"));
    assert!(events.iter().any(|event| {
        event["event"] == "result"
            && event["status"] == "success"
            && event["data"]["processed_records"] == 20
    }));
}
