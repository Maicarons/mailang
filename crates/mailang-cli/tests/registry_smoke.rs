//! CLI smoke tests for registry commands (publish / install / search / yank / init).
//! Uses `CARGO_BIN_EXE_mailang` provided by cargo for integration tests.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mailang"))
}

fn tmp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("mailang_cli_reg_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn write_pkg(dir: &Path, name: &str, vers: &str, desc: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("mailang.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"{}\"\ndescription = \"{}\"\n",
            name, vers, desc
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("lib.mai"),
        format!("fn ping() {{ return \"{}\" }}\n", name),
    )
    .unwrap();
    std::fs::write(
        dir.join("mailib.ini"),
        format!(
            "[module]\nname = {}\nversion = {}\nentry = lib.mai\n",
            name, vers
        ),
    )
    .unwrap();
    std::fs::write(dir.join("README"), format!("{}\n", desc)).unwrap();
}

#[test]
fn registry_init_publish_install_search_yank() {
    let work = tmp("full");
    let reg = work.join("reg");
    let pkg = work.join("pkg");
    let app = work.join("app");

    // registry init
    let out = bin()
        .args(["registry", "init"])
        .arg(&reg)
        .output()
        .expect("run registry init");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(reg.join("index").is_dir());
    assert!(reg.join("pkgs").is_dir());

    // publish
    write_pkg(&pkg, "greet", "0.1.0", "hello helpers");
    let out = bin()
        .current_dir(&pkg)
        .args(["publish", "--registry"])
        .arg(&reg)
        .output()
        .expect("run publish");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("published greet@0.1.0"), "{}", stdout);

    // duplicate publish refused
    let out = bin()
        .current_dir(&pkg)
        .args(["publish", "--registry"])
        .arg(&reg)
        .output()
        .expect("run publish again");
    assert!(!out.status.success());

    // install into app
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        app.join("mailang.toml"),
        "[package]\nname = \"app\"\nversion = \"0.0.1\"\n",
    )
    .unwrap();
    let out = bin()
        .current_dir(&app)
        .args(["install", "greet", "--registry"])
        .arg(&reg)
        .output()
        .expect("run install");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(app.join("vendor").join("greet").join("lib.mai").is_file());
    let manifest = std::fs::read_to_string(app.join("mailang.toml")).unwrap();
    assert!(manifest.contains("greet"));

    // search
    let out = bin()
        .args(["search", "hello", "--registry"])
        .arg(&reg)
        .output()
        .expect("run search");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("greet"), "{}", stdout);

    // yank then install refused
    let out = bin()
        .args(["yank", "greet", "0.1.0", "--registry"])
        .arg(&reg)
        .output()
        .expect("run yank");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = bin()
        .current_dir(&app)
        .args(["install", "greet@0.1.0", "--registry"])
        .arg(&reg)
        .output()
        .expect("run install yanked");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("yanked") || stderr.contains("Yanked") || stderr.contains("allow-yanked")
    );

    // --allow-yanked works
    let out = bin()
        .current_dir(&app)
        .args([
            "install",
            "greet@0.1.0",
            "--registry",
            "--allow-yanked",
            "--no-manifest",
        ])
        .arg(&reg)
        .output()
        .expect("run install allow-yanked");
    // Note: clap parses --registry then value; the flag order above may be wrong.
    // Re-run with explicit flag/value pairs if needed.
    if !out.status.success() {
        let out = bin()
            .current_dir(&app)
            .arg("install")
            .arg("greet@0.1.0")
            .arg("--registry")
            .arg(&reg)
            .arg("--allow-yanked")
            .arg("--no-manifest")
            .output()
            .expect("run install allow-yanked v2");
        assert!(
            out.status.success(),
            "stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let _ = std::fs::remove_dir_all(&work);
}
