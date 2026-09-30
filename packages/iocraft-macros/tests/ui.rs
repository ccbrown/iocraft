use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn check_fixture(project: &Path, name: &str, expected: Option<(&str, &[&str])>) {
    let output = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--message-format=json", "--bin", name])
        .arg("--manifest-path")
        .arg(project.join("Cargo.toml"))
        // Do not contend with the outer cargo invocation's target-directory lock.
        .arg("--target-dir")
        .arg(project.join("target"))
        .output()
        .expect("failed to run cargo check for UI fixture");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| {
            message["reason"] == "compiler-message" && message["message"]["level"] == "error"
        })
        .collect::<Vec<_>>();
    let context = format!("fixture {name}:\n{stdout}\n{stderr}");

    if let Some((code, fragments)) = expected {
        assert!(
            !output.status.success(),
            "expected a compile error: {context}"
        );
        assert_eq!(errors.len(), 1, "expected one compile error: {context}");
        assert_eq!(errors[0]["target"]["name"], name, "{context}");
        assert_eq!(errors[0]["message"]["code"]["code"], code, "{context}");
        let message = errors[0]["message"]["message"].as_str().unwrap();
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "missing {fragment:?}: {context}"
            );
        }
    } else {
        assert!(
            output.status.success(),
            "expected successful compilation: {context}"
        );
        assert!(errors.is_empty(), "{context}");
    }
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

    check_fixture(project.path(), "complete_required_props", None);
    // Only assert the diagnostic's meaning. Rendered notes, source snippets,
    // qualification and wrapping can differ between compiler versions/platforms.
    check_fixture(
        project.path(),
        "missing_required_prop",
        Some((
            "E0599",
            &[
                "__iocraft_build",
                "__IocraftExamplePropsBuilder",
                "MissingRequiredProperty_important_value",
            ],
        )),
    );
    check_fixture(
        project.path(),
        "private_builder_fields",
        Some((
            "E0616",
            &["`value`", "__IocraftPublicPropsBuilder", "private"],
        )),
    );
}
