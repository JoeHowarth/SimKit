use std::process::Command;

fn stitchlands_bin() -> String {
    // Cargo exposes the compiled bin path in this env var for tests
    env!("CARGO_BIN_EXE_stitchlands").to_string()
}

#[test]
fn headless_exits_after_n_ticks() {
    let exe = stitchlands_bin();
    let output = Command::new(exe)
        .args(["--mode", "headless", "--ticks", "5", "--seed", "1"])
        .output()
        .expect("run stitchlands");
    assert!(output.status.success(), "process should exit 0");
}

#[test]
fn determinism_empty_world_snapshot_hash() {
    let exe = stitchlands_bin();
    let run = |seed: u64| {
        let output = Command::new(&exe)
            .args([
                "--mode",
                "headless",
                "--ticks",
                "5",
                "--seed",
                &seed.to_string(),
            ])
            .output()
            .expect("run stitchlands");
        assert!(output.status.success(), "process should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout
            .lines()
            .find(|l| l.starts_with("SNAP:"))
            .expect("SNAP line");
        line.trim().to_string()
    };

    let h1 = run(1);
    let h2 = run(1);
    assert_eq!(h1, h2, "hashes should match");
}
