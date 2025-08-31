use std::process::Command;

fn stitchlands_bin() -> String {
    env!("CARGO_BIN_EXE_stitchlands").to_string()
}

#[test]
fn headless_loads_scenario_and_exits() {
    let exe = stitchlands_bin();
    let scenario_path = format!(
        "{}/../assets/scenarios/phase0/basic.ron",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = Command::new(&exe)
        .args([
            "--mode",
            "headless",
            "--ticks",
            "3",
            "--seed",
            "1",
            "--scenario",
            &scenario_path,
        ])
        .output()
        .expect("run stitchlands");
    if !output.status.success() {
        eprintln!(
            "stitchlands failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(output.status.success(), "process should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let snap = stdout
        .lines()
        .find(|l| l.starts_with("SNAP:"))
        .expect("SNAP line present");
    assert!(snap.len() > 5);
}

#[test]
fn determinism_with_same_scenario_is_stable() {
    let exe = stitchlands_bin();
    let scenario_path = format!(
        "{}/../assets/scenarios/phase0/basic.ron",
        env!("CARGO_MANIFEST_DIR")
    );
    let run = || {
        let output = Command::new(&exe)
            .args([
                "--mode",
                "headless",
                "--ticks",
                "3",
                "--seed",
                "1",
                "--scenario",
                &scenario_path,
            ])
            .output()
            .expect("run stitchlands");
        if !output.status.success() {
            eprintln!(
                "stitchlands failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(output.status.success(), "process should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find(|l| l.starts_with("SNAP:"))
            .unwrap()
            .to_string()
    };

    let h1 = run();
    let h2 = run();
    assert_eq!(h1, h2, "hashes should match across runs");
}
