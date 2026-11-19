//! `nu/fff.nu` against a real server on a fixture index, run in a real
//! `nu`. Skipped when `nu` isn't installed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use fffinder::server::{self, Options, UiRequest};
use fffinder_core::{Engine, EngineOptions};
use serde_json::{Value, json};

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("fff-nu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("tree/Projects/atlas")).unwrap();
        std::fs::write(dir.join("tree/Roadmap.md"), "# Roadmap\n").unwrap();
        std::fs::write(
            dir.join("tree/notes.txt"),
            "plain notes\nTODO: atlas maps\n",
        )
        .unwrap();
        std::fs::write(dir.join("tree/Projects/atlas/Project Atlas.txt"), "atlas\n").unwrap();
        Fixture { dir }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn module() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("nu/fff.nu")
}

/// Run `script` with fff.nu loaded; returns stdout parsed as JSON.
fn nu(nu: &Path, data_dir: &Path, cwd: &Path, script: &str) -> Result<Value, String> {
    let full = format!(
        "use {} *; {script} | to json --raw",
        serde_json::to_string(&module()).unwrap()
    );
    let out = Command::new(nu)
        .args(["--no-config-file", "-c", &full])
        .env("FFFINDER_DATA_DIR", data_dir)
        .env("NO_COLOR", "1")
        .current_dir(cwd)
        .output()
        .unwrap();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("{e}: {}", String::from_utf8_lossy(&out.stdout)))
}

/// A shell script named `name` to run in fffinder's place: the real program
/// would open a window.
#[cfg(unix)]
fn stand_in(fx: &Fixture, name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = fx.dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The commands that start fffinder hand `--here` on to it. The stand-in
/// writes down the arguments it is started with.
#[cfg(unix)]
#[test]
fn fff_nu_commands_pass_here_to_fffinder() {
    let Some(nu_bin) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("here");
    let script = "printf '%s\\n' \"$@\" > \"$0.args\"\nprintf '[]'";
    let stand_in = stand_in(&fx, "fffinder", script);
    let noted = fx.dir.join("fffinder.args");
    // The commands spell the folder with its symlinks resolved.
    let tree = std::fs::canonicalize(fx.dir.join("tree")).unwrap();
    let dir = tree.to_str().unwrap();
    let run = |script: &str| {
        let script = format!(
            "$env.FFFINDER_BIN = {}; {script}",
            serde_json::to_string(&stand_in).unwrap()
        );
        nu(&nu_bin, &fx.dir, &tree, &script).unwrap_or_else(|e| panic!("{script}:\n{e}"))
    };
    let started_with = |script: &str| -> Vec<String> {
        let _ = std::fs::remove_file(&noted);
        run(script);
        let args = std::fs::read_to_string(&noted).unwrap_or_else(|e| panic!("{script}: {e}"));
        args.lines().map(str::to_string).collect()
    };

    assert_eq!(
        started_with("fff gui . --query readme --here"),
        ["--detach", dir, "--query", "readme", "--here"]
    );
    assert_eq!(started_with("fff gui"), ["--detach", dir]);
    assert_eq!(
        started_with("fff pick --here budget"),
        ["--pick", dir, "--here", "--query", "budget"]
    );
    assert_eq!(
        started_with("fff pick budget"),
        ["--pick", dir, "--query", "budget"]
    );

    // The commands built on `fff pick` hand it on, and only when given.
    assert_eq!(
        started_with("fff insert --here"),
        ["--pick", dir, "--multiple", "--here"]
    );
    assert_eq!(started_with("fff insert"), ["--pick", dir, "--multiple"]);
    assert_eq!(
        started_with("fff cd --here budget"),
        ["--pick", dir, "--here", "--query", "budget"]
    );
    assert_eq!(
        started_with("fff cd budget"),
        ["--pick", dir, "--query", "budget"]
    );
    assert_eq!(
        started_with("fff edit --here budget"),
        ["--pick", dir, "--here", "--query", "budget"]
    );

    // Ctrl+T and Alt+C run them with it.
    let bound = "$env.config.keybindings | where name in [fff_insert fff_cd] | get event.cmd";
    assert_eq!(
        run(&format!("fff keybindings --here; {bound}")),
        json!(["fff insert --here", "fff cd --here"])
    );
    assert_eq!(
        run(&format!("fff keybindings; {bound}")),
        json!(["fff insert", "fff cd"])
    );
}

/// An fffinder older than the module doesn't know `--here`: it says so and
/// leaves without showing a chooser. `fff pick` passes that on, where a
/// chooser that was cancelled gives nothing.
#[cfg(unix)]
#[test]
fn fff_pick_says_when_fffinder_refuses_an_option() {
    let Some(nu_bin) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("refused");
    let tree = fx.dir.join("tree");
    let run = |app: &Path, script: &str| {
        let app = serde_json::to_string(app).unwrap();
        nu(
            &nu_bin,
            &fx.dir,
            &tree,
            &format!("$env.FFFINDER_BIN = {app}; {script}"),
        )
    };

    let older = "echo 'fffinder: unknown option: --here' >&2\necho 'Usage: fffinder' >&2\nexit 2";
    let older = stand_in(&fx, "older", older);
    for command in ["fff pick --here", "fff insert --here", "fff cd --here"] {
        let said = run(&older, command).expect_err(command);
        assert!(
            said.contains("fffinder: unknown option: --here"),
            "{command}: {said}"
        );
        assert!(!said.contains("Usage:"), "only what went wrong: {said}");
    }

    // Escape and Cancel leave with 1, and that is no error.
    let cancelled = stand_in(&fx, "cancelled", "exit 1");
    assert_eq!(run(&cancelled, "fff pick --here"), Ok(Value::Null));
    assert_eq!(run(&cancelled, "fff pick --multiple"), Ok(json!([])));
}

#[test]
fn fff_nu_commands_talk_to_the_window() {
    let Some(nu_bin) = fffinder::nushell::find_nu(None) else {
        eprintln!("nu not installed; skipping");
        return;
    };
    let fx = Fixture::new("module");
    let tree = fx.dir.join("tree");
    let data = fx.dir.join("data");
    let engine: Arc<Engine> = Engine::new(EngineOptions {
        roots: vec![tree.clone()],
        app_dirs: Vec::new(),
        db_dir: None,
        watch: false,
        search: Default::default(),
    });
    assert!(engine.wait_ready(Duration::from_secs(20)));
    let (tx, rx) = async_channel::unbounded::<UiRequest>();
    let _server = server::start_with(
        engine,
        Some(tx),
        Options {
            run_dir: data.join("run"),
            tcp: cfg!(windows),
        },
    )
    .unwrap();
    // A stand-in for the window.
    std::thread::spawn(move || {
        while let Ok(request) = rx.recv_blocking() {
            match request {
                UiRequest::Selection { all, reply } => {
                    let _ = reply.send(Ok(json!({
                        "location": "folder",
                        "items": [{"path": "/x/a.txt", "name": "a.txt", "type": "file", "size": 3, "modified": 1_700_000_000, "all": all}]
                    })));
                }
                UiRequest::Show {
                    paths,
                    title,
                    reply,
                } => {
                    let _ = reply.send(Ok(json!({"shown": paths.len(), "missing": [], "title": title, "paths": paths})));
                }
                UiRequest::Open { reply, .. } => {
                    let _ = reply.send(Ok(json!({"ok": true})));
                }
            }
        }
    });
    let run = |script: &str| {
        nu(&nu_bin, &data, &tree, script).unwrap_or_else(|e| panic!("{script}:\n{e}"))
    };

    assert_eq!(run("fff status | get files"), 3);
    let found = run("fff find atlas");
    let rows = found.as_array().unwrap();
    assert!(
        rows.iter()
            .any(|r| r["name"] == "Project Atlas.txt" && r["type"] == "file"),
        "{found}"
    );
    assert!(
        rows.iter()
            .any(|r| r["name"] == "atlas" && r["type"] == "dir"),
        "{found}"
    );
    assert_eq!(
        run("fff find atlas --files | get type | uniq"),
        json!(["file"])
    );
    assert_eq!(
        run("fff find atlas --dirs | get type | uniq"),
        json!(["dir"])
    );
    assert_eq!(
        run("fff find notes --here --limit 1 | get 0.name"),
        "notes.txt"
    );
    // Sizes and dates arrive as Nushell types.
    assert_eq!(
        run("fff find notes --files | get 0.size | describe"),
        "filesize"
    );
    assert_eq!(
        run("fff find notes --files | get 0.modified | describe"),
        "datetime"
    );

    assert_eq!(run("fff files | length"), 3);
    assert_eq!(run("fff files notes | get 0.name"), "notes.txt");
    let dirs = run("fff dirs | get name");
    assert!(dirs.as_array().unwrap().contains(&json!("atlas")), "{dirs}");

    let hits = run("fff search 'atlas maps'");
    assert_eq!(hits[0]["line_number"], 2);
    assert!(hits[0]["path"].as_str().unwrap().ends_with("notes.txt"));
    let context = run("fff search 'atlas maps' --context 1 | get 0.context");
    assert_eq!(context, json!(["plain notes", "TODO: atlas maps"]));
    assert_eq!(run("fff search --regex 'TO.O' Roadmap | length"), 2);

    let selection = run("fff selection");
    assert_eq!(selection[0]["name"], "a.txt");
    assert_eq!(
        run("fff selection --all | get 0.size | describe"),
        "filesize"
    );

    let shown = run("ls | fff show --title Mine");
    assert_eq!(shown["shown"], 3);
    assert_eq!(shown["title"], "Mine");
    assert!(
        shown["paths"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| Path::new(p.as_str().unwrap()).is_absolute()),
        "relative names from ls are expanded: {shown}"
    );
    assert_eq!(run("['notes.txt'] | fff show | get shown"), 1);

    // Without a window, every command says so.
    let none = fx.dir.join("nowhere");
    std::fs::create_dir_all(&none).unwrap();
    let err = nu(&nu_bin, &none, &tree, "fff find x").unwrap_err();
    assert!(err.contains("no fffinder window is running"), "{err}");
    let err = nu(&nu_bin, &data, &tree, "fff watch").unwrap_err();
    assert!(err.contains("--timeout or --max-events"), "{err}");
}
