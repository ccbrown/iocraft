use std::{fs, path::Path, process::Command};

fn check_fixture(project: &Path, name: &str, should_compile: bool) {
    let output = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--bin", name])
        .arg("--manifest-path")
        .arg(project.join("Cargo.toml"))
        // Do not contend with the outer cargo invocation's target-directory lock.
        .arg("--target-dir")
        .arg(project.join("target"))
        .output()
        .expect("failed to run cargo check for UI fixture");
    assert_eq!(
        output.status.success(),
        should_compile,
        "fixture {name}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn props_diagnostics() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project = tempfile::tempdir().unwrap();
    let bins = project.path().join("src/bin");
    fs::create_dir_all(&bins).unwrap();
    for name in [
        "missing_required_prop",
        "private_builder_fields",
        "complete_required_props",
    ] {
        fs::copy(
            manifest_dir.join(format!("tests/ui/{name}.rs")),
            bins.join(format!("{name}.rs")),
        )
        .unwrap();
    }
    let iocraft = manifest_dir.join("../iocraft").canonicalize().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"iocraft-props-ui\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\niocraft = {{ path = {} }}\n",
            serde_json::to_string(&iocraft).unwrap()
        ),
    ).unwrap();
    // Reuse the dependency versions selected by the outer workspace build.
    fs::copy(
        manifest_dir.join("../../Cargo.lock"),
        project.path().join("Cargo.lock"),
    )
    .unwrap();

    // Check consumer compilation boundaries, not compiler prose or generated builder names.
    check_fixture(project.path(), "complete_required_props", true);
    check_fixture(project.path(), "missing_required_prop", false);
    check_fixture(project.path(), "private_builder_fields", false);
}
