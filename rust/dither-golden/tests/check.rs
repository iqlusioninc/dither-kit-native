//! --check must validate the committed artifacts as well as VM/upstream parity.
use std::{fs, path::PathBuf, process::Command};

struct Fixtures(PathBuf);
impl Drop for Fixtures {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixtures() -> Fixtures {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let dir = std::env::temp_dir().join(format!("dither-golden-check-{}", std::process::id()));
    fs::create_dir_all(dir.join("png")).unwrap();
    fs::copy(source.join("scenes.json"), dir.join("scenes.json")).unwrap();
    for entry in fs::read_dir(source.join("png")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.join("png").join(entry.file_name())).unwrap();
    }
    Fixtures(dir)
}

fn check(f: &Fixtures) -> bool {
    Command::new(env!("CARGO_BIN_EXE_dither-golden"))
        .arg(&f.0)
        .arg("--check")
        .output()
        .unwrap()
        .status
        .success()
}

#[test]
fn check_rejects_stale_or_missing_artifacts_without_writing() {
    let f = fixtures();
    assert!(check(&f), "original fixtures should pass");
    let path = f.0.join("scenes.json");
    let original = fs::read(&path).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["scenes"][0]["program"][1] = serde_json::json!(999.0);
    let changed = serde_json::to_vec(&json).unwrap();
    fs::write(&path, &changed).unwrap();
    assert!(!check(&f), "a changed stored program must fail --check");
    assert_eq!(
        fs::read(&path).unwrap(),
        changed,
        "--check must not rewrite JSON"
    );
    fs::write(&path, original).unwrap();
    let png = f.0.join("png/gradient_0.png");
    let original_png = fs::read(&png).unwrap();
    fs::copy(f.0.join("png/gradient_1.png"), &png).unwrap();
    assert!(!check(&f), "a different PNG must fail --check");
    fs::write(&png, original_png).unwrap();
    fs::remove_file(&png).unwrap();
    assert!(!check(&f), "a missing PNG must fail --check");
    assert!(!png.exists(), "--check must not regenerate missing PNGs");
}
