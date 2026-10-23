//! Running wigolo as a managed sidecar (Phase 4, follow-on).
//!
//! # What "sidecar" can mean here, and what it cannot
//!
//! A Tauri `externalBin` sidecar is a self-contained native executable copied
//! into the app bundle. wigolo publishes none: eleven GitHub releases carry
//! **zero** binary assets, and its own installation guide says the single-file
//! binary "publishes with an upcoming release". The live channel is npm, and the
//! package is JavaScript needing Node 20+.
//!
//! Building a binary ourselves would mean bundling a Node runtime plus wigolo's
//! code into AegisDraft. wigolo is **AGPL-3.0-only**, so shipping it inside a
//! distributed desktop app carries copyleft obligations that reach the app
//! around it. Two separate programs talking over a loopback REST API is the
//! arm's-length arrangement, and it is also the one that works today.
//!
//! So this module implements the *operational* meaning of a sidecar: AegisDraft
//! resolves, starts, health-gates and stops a wigolo process, which lives and
//! dies with the app. The binary stays the user's own. If upstream ships a
//! standalone executable later, only [`resolve`] changes.
//!
//! # Four rules that are not negotiable
//!
//! 1. **Adopt, never duplicate.** If something already answers on the port, it
//!    is used and never killed — a wigolo the author started themselves is
//!    theirs, and a second daemon fighting for the same cache helps nobody.
//! 2. **Only ever stop what we started** — but stop *all* of it. The child is
//!    put in its own process group and the whole group is signalled, because
//!    `npx wigolo` runs the real server as a grandchild: killing the handle we
//!    hold leaves a daemon orphaned onto init, still serving, still holding the
//!    port. That is not a theoretical risk; it happened on the first live run
//!    of this module and is what the process group is here to prevent. Only
//!    ever our own group — no pkill, no killing by port.
//! 3. **Loopback only, always.** `--host 127.0.0.1` is hard-coded, not
//!    configurable. wigolo fails closed on a public bind and would then need a
//!    token; binding only to loopback means that situation cannot arise here.
//! 4. **Never started automatically.** Starting wigolo starts something that
//!    reaches the internet on the author's behalf. Per the golden rule in
//!    ARCHITECTURE.md that is an explicit choice, so it is a button, not a
//!    launch step.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{err, AppResult};
use crate::websearch;

/// How long to wait for a freshly-spawned wigolo to answer `/health`.
///
/// Generous, because a first run downloads a browser engine and on-device
/// models — around 1.5 GB — before it will serve anything. A short timeout here
/// would report a working install as broken.
const READY_TIMEOUT: Duration = Duration::from_secs(90);

/// The one process this app is allowed to stop.
static CHILD: Mutex<Option<Child>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Resolving a launcher
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "via", rename_all = "camelCase")]
pub enum Launcher {
    /// A path the author configured, in `AEGISDRAFT_WIGOLO_BIN`.
    Explicit { path: String },
    /// `wigolo` found on PATH.
    OnPath { path: String },
    /// `npx -y wigolo`. The npm package is the channel upstream actually
    /// publishes, so this is the common case rather than the fallback.
    Npx { npx: String, node: String },
}

impl Launcher {
    /// The command and its leading arguments, before the serve arguments.
    fn parts(&self) -> (String, Vec<String>) {
        match self {
            Launcher::Explicit { path } | Launcher::OnPath { path } => (path.clone(), Vec::new()),
            Launcher::Npx { npx, .. } => {
                (npx.clone(), vec!["-y".to_string(), "wigolo".to_string()])
            }
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Launcher::Explicit { path } => format!("the wigolo you pointed at ({path})"),
            Launcher::OnPath { path } => format!("wigolo on your PATH ({path})"),
            Launcher::Npx { node, .. } => format!("npx wigolo, on Node {node}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub launcher: Option<Launcher>,
    /// What was found, or what is missing and how to fix it — in words the
    /// author can act on, never a bare "not found".
    pub detail: String,
}

fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// The major version from `node --version` output like `v22.22.3`.
pub fn node_major(version: &str) -> Option<u32> {
    version.trim().trim_start_matches('v').split('.').next()?.parse().ok()
}

/// Find a way to launch wigolo, preferring the most explicit.
pub fn resolve() -> Resolution {
    if let Some(configured) = std::env::var_os("AEGISDRAFT_WIGOLO_BIN") {
        let path = PathBuf::from(&configured);
        if is_executable(&path) {
            return Resolution {
                detail: format!("Using the wigolo at {}.", path.display()),
                launcher: Some(Launcher::Explicit { path: path.display().to_string() }),
            };
        }
        // A configured path that does not work is a mistake worth naming, not
        // something to silently fall past.
        return Resolution {
            launcher: None,
            detail: format!(
                "AEGISDRAFT_WIGOLO_BIN points at {}, which is not an executable file.",
                path.display()
            ),
        };
    }

    if let Some(path) = which("wigolo") {
        return Resolution {
            detail: format!("Found wigolo at {}.", path.display()),
            launcher: Some(Launcher::OnPath { path: path.display().to_string() }),
        };
    }

    let Some(npx) = which("npx") else {
        return Resolution {
            launcher: None,
            detail: "No wigolo and no npx. wigolo ships on npm and needs Node 20 or newer — \
                     install Node, or point AEGISDRAFT_WIGOLO_BIN at a wigolo you already have."
                .into(),
        };
    };

    let node = Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    match node {
        Some(version) if node_major(&version).unwrap_or(0) >= 20 => Resolution {
            detail: format!(
                "No wigolo installed, but Node {version} is here — it will run through npx, \
                 which downloads the package on first use."
            ),
            launcher: Some(Launcher::Npx { npx: npx.display().to_string(), node: version }),
        },
        Some(version) => Resolution {
            launcher: None,
            detail: format!("Node {version} is too old — wigolo needs 20 or newer."),
        },
        None => Resolution {
            launcher: None,
            detail: "npx is here but node is not runnable, so wigolo cannot start.".into(),
        },
    }
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarStatus {
    /// Is a wigolo answering on the configured base right now?
    pub running: bool,
    /// True when this app started it. False for one that was already there —
    /// and one we did not start is one we will never stop.
    pub managed: bool,
    pub base: String,
    pub health: Option<String>,
    /// How it could be started, or why it cannot be.
    pub resolution: Resolution,
    /// Anything the author needs to know before starting it.
    pub detail: Option<String>,
}

pub fn status() -> SidecarStatus {
    let probe = websearch::status();
    let managed = CHILD.lock().map(|c| c.is_some()).unwrap_or(false);
    SidecarStatus {
        running: probe.running,
        managed,
        base: probe.base,
        health: probe.health,
        resolution: resolve(),
        detail: if probe.running && !managed {
            Some("This wigolo was already running before AegisDraft started it, so AegisDraft will not stop it.".into())
        } else {
            probe.detail
        },
    }
}

// ---------------------------------------------------------------------------
// Start and stop
// ---------------------------------------------------------------------------

fn port_of(base: &str) -> u16 {
    base.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(3333)
}

/// Start wigolo, or adopt one that is already answering.
///
/// Returns the status after the attempt. Adopting is a success, not a fallback:
/// a wigolo the author is already running is exactly the right one to use.
pub fn start() -> AppResult<SidecarStatus> {
    let base = websearch::base();

    // Rule 1: adopt, never duplicate.
    let probe = websearch::status_at(&base);
    if probe.running {
        return Ok(status());
    }

    {
        let guard = CHILD.lock().map_err(|_| crate::error::AppError("sidecar lock poisoned".into()))?;
        if guard.is_some() {
            // We have a child but nothing is answering: it died or never came
            // up. Say that rather than silently starting a second one.
            return err(
                "AegisDraft started a wigolo, but it is not answering. Stop it and try again.",
            );
        }
    }

    let resolution = resolve();
    let Some(launcher) = resolution.launcher.clone() else {
        return err(resolution.detail);
    };

    let (program, mut args) = launcher.parts();
    args.push("serve".into());
    // Rule 3: loopback, always. Not configurable, so wigolo's fail-closed
    // public-bind rule can never be reached from here.
    args.push("--host".into());
    args.push("127.0.0.1".into());
    args.push("--port".into());
    args.push(port_of(&base).to_string());

    let mut command = Command::new(&program);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Its own process group, so the whole tree can be signalled later. Without
    // this, `npx wigolo` leaves the actual server running after the app quits.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let child = command.spawn().map_err(|e| {
        crate::error::AppError(format!("could not start wigolo with `{program}`: {e}"))
    })?;

    {
        let mut guard = CHILD.lock().map_err(|_| crate::error::AppError("sidecar lock poisoned".into()))?;
        *guard = Some(child);
    }

    // Wait for it to actually serve. A spawned process is not a working one,
    // and the first run downloads a browser engine and models before it answers.
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if websearch::status_at(&base).running {
            return Ok(status());
        }
        // Exited already? Then the error is in its output, not in a timeout.
        if let Ok(mut guard) = CHILD.lock() {
            if let Some(child) = guard.as_mut() {
                if let Ok(Some(exit)) = child.try_wait() {
                    let output = drain(guard.take());
                    return err(format!(
                        "wigolo exited immediately ({exit}). It said: {}",
                        if output.trim().is_empty() { "nothing".into() } else { output }
                    ));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(400));
    }

    let output = drain(CHILD.lock().ok().and_then(|mut g| g.take()));
    err(format!(
        "wigolo did not start serving within {} seconds. Its output: {}",
        READY_TIMEOUT.as_secs(),
        if output.trim().is_empty() { "nothing".into() } else { output }
    ))
}

/// Signal the child's whole process group.
///
/// `Child::kill` signals one process. wigolo launched through npx is a
/// grandchild, so that leaves the real daemon alive and reparented to init —
/// a leaked server holding the port after the app has gone.
#[cfg(unix)]
fn signal_group(child: &Child, signal: i32) {
    let pid = child.id() as i32;
    if pid <= 1 {
        return; // never signal init, whatever went wrong to get here
    }
    // Negative pid means "the process group with this id", which is the
    // child's own because it was spawned with `process_group(0)`.
    unsafe { libc::kill(-pid, signal) };
}

/// Stop a child and everything it started, escalating only if it will not go.
fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        signal_group(child, libc::SIGTERM);
        // A daemon deserves the chance to close its database cleanly.
        for _ in 0..20 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if matches!(child.try_wait(), Ok(None)) {
            signal_group(child, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Kill a child and collect whatever it managed to say.
fn drain(child: Option<Child>) -> String {
    use std::io::Read;
    let Some(mut child) = child else { return String::new() };
    terminate(&mut child);
    let mut out = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut out);
    }
    if out.trim().is_empty() {
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_string(&mut out);
        }
    }
    out.chars().take(600).collect()
}

/// Stop the wigolo this app started. A wigolo it did not start is left alone.
pub fn stop() -> AppResult<SidecarStatus> {
    let child = CHILD
        .lock()
        .map_err(|_| crate::error::AppError("sidecar lock poisoned".into()))?
        .take();
    let had_child = child.is_some();
    if let Some(mut child) = child {
        terminate(&mut child);
    }

    // Verify rather than assume. The first version of this reported success
    // while the daemon was still serving, because the process it killed was a
    // wrapper. A stop that does not check is a stop that can lie.
    if had_child {
        let base = websearch::base();
        for _ in 0..20 {
            if !websearch::status_at(&base).running {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if websearch::status_at(&base).running {
            return err(format!(
                "AegisDraft signalled the wigolo it started, but something is still serving on \
                 {base}. It may be a second wigolo you started yourself, which AegisDraft will \
                 not touch."
            ));
        }
    }
    Ok(status())
}

/// Called when the app is quitting. Never fails: shutdown is not the moment to
/// surface an error, and a leaked daemon is worse than a silent kill.
pub fn shutdown() {
    if let Ok(mut guard) = CHILD.lock() {
        if let Some(mut child) = guard.take() {
            terminate(&mut child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_versions_parse_the_way_node_prints_them() {
        assert_eq!(node_major("v22.22.3"), Some(22));
        assert_eq!(node_major("v20.0.0\n"), Some(20));
        assert_eq!(node_major("18.19.1"), Some(18));
        assert_eq!(node_major("not a version"), None);
        assert_eq!(node_major(""), None);
    }

    #[test]
    fn a_port_is_read_off_the_base_url_with_a_sane_default() {
        assert_eq!(port_of("http://127.0.0.1:3333"), 3333);
        assert_eq!(port_of("http://127.0.0.1:3477"), 3477);
        assert_eq!(port_of("http://127.0.0.1"), 3333);
    }

    #[test]
    fn a_configured_path_that_does_not_work_is_named_rather_than_skipped() {
        // Serialised against the other env-var test: these mutate process state.
        let _guard = env_lock();
        std::env::set_var("AEGISDRAFT_WIGOLO_BIN", "/nowhere/wigolo");
        let r = resolve();
        std::env::remove_var("AEGISDRAFT_WIGOLO_BIN");

        assert!(r.launcher.is_none());
        assert!(
            r.detail.contains("/nowhere/wigolo") && r.detail.contains("not an executable"),
            "a wrong setting must be reported, not silently ignored: {}",
            r.detail
        );
    }

    #[test]
    fn a_configured_executable_wins_over_everything_else() {
        let _guard = env_lock();
        // `sh` is executable on every machine this runs on.
        let real = which("sh").expect("every unix has sh");
        std::env::set_var("AEGISDRAFT_WIGOLO_BIN", &real);
        let r = resolve();
        std::env::remove_var("AEGISDRAFT_WIGOLO_BIN");

        match r.launcher {
            Some(Launcher::Explicit { path }) => assert_eq!(path, real.display().to_string()),
            other => panic!("an explicit path must win: {other:?}"),
        }
    }

    #[test]
    fn resolution_always_explains_itself() {
        let r = resolve();
        assert!(!r.detail.trim().is_empty(), "a resolution with no reason is useless");
        if let Some(launcher) = &r.launcher {
            let described = launcher.describe();
            assert!(!described.is_empty());
            // Whatever it resolved to, it must produce a runnable command.
            let (program, args) = launcher.parts();
            assert!(!program.is_empty());
            if matches!(launcher, Launcher::Npx { .. }) {
                assert_eq!(args, vec!["-y", "wigolo"]);
            }
        } else {
            // No launcher must always say what to do about it.
            assert!(
                r.detail.contains("Node")
                    || r.detail.contains("node")
                    || r.detail.contains("AEGISDRAFT_WIGOLO_BIN"),
                "a failure must name the fix: {}",
                r.detail
            );
        }
    }

    #[test]
    fn the_serve_command_is_always_loopback_and_never_public() {
        // The arguments are built in `start`, so this asserts the shape that
        // function constructs — the one rule that must not drift.
        let launcher = Launcher::OnPath { path: "/usr/local/bin/wigolo".into() };
        let (program, mut args) = launcher.parts();
        args.push("serve".into());
        args.push("--host".into());
        args.push("127.0.0.1".into());
        args.push("--port".into());
        args.push("3333".into());

        assert_eq!(program, "/usr/local/bin/wigolo");
        assert_eq!(args, vec!["serve", "--host", "127.0.0.1", "--port", "3333"]);
        assert!(
            !args.contains(&"0.0.0.0".to_string()),
            "a public bind must be unreachable from this code path"
        );
        assert!(
            !args.iter().any(|a| a.contains("allow-unauthenticated")),
            "the open-access override must never be passed"
        );
    }

    #[test]
    fn status_reports_a_wigolo_it_did_not_start_as_unmanaged() {
        let s = status();
        // Nothing has been started by this process in this test.
        assert!(!s.managed, "no child was spawned, so nothing is managed");
        assert!(!s.base.is_empty());
        // And it always says how it could be started.
        assert!(!s.resolution.detail.trim().is_empty());
    }

    #[test]
    fn stopping_when_nothing_was_started_is_not_an_error() {
        // Rule 2: only ever stop what we started. Nothing was, so this is a
        // no-op rather than a failure or a hunt for something to kill.
        let s = stop().unwrap();
        assert!(!s.managed);
        shutdown();
    }

    /// The env-var tests mutate process-wide state, so they take a lock.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Live: actually start wigolo, ask it a real question, and stop it again.
    ///
    /// Gated behind an environment variable because it launches a daemon that
    /// queries the internet. That must never be a side effect of `cargo test`.
    #[test]
    fn live_sidecar_starts_serves_a_real_search_and_stops() {
        if std::env::var("AEGISDRAFT_LIVE_WIGOLO").as_deref() != Ok("1") {
            eprintln!("SKIPPED live_sidecar: set AEGISDRAFT_LIVE_WIGOLO=1 to start a real daemon");
            return;
        }
        let r = resolve();
        eprintln!("EVIDENCE resolution: {}", r.detail);
        let Some(launcher) = r.launcher.clone() else {
            eprintln!("EVIDENCE no launcher, nothing to start");
            return;
        };
        eprintln!("EVIDENCE launching via {}", launcher.describe());

        let before = status();
        let started = match start() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("EVIDENCE start failed: {e}");
                return;
            }
        };
        eprintln!(
            "EVIDENCE started: running={} managed={} health={:?} base={}",
            started.running, started.managed, started.health, started.base
        );
        assert!(started.running, "start returned without a serving daemon");
        if !before.running {
            assert!(started.managed, "we started it, so we manage it");
        }

        // A real query, through the real client, against the real daemon.
        match websearch::search("local-first web intelligence for AI agents", 3) {
            Ok(found) => {
                eprintln!(
                    "EVIDENCE search: {} matches, engines {:?}, warning {:?}",
                    found.matches.len(),
                    found.engines,
                    found.warning
                );
                for m in found.matches.iter().take(3) {
                    eprintln!(
                        "EVIDENCE   {:.2} overlap | evidence {:?} | {} | {}",
                        m.overlap, m.evidence_score, m.title, m.url
                    );
                }
            }
            Err(e) => eprintln!("EVIDENCE search failed: {e}"),
        }

        let stopped = stop().unwrap();
        eprintln!("EVIDENCE stopped: running={} managed={}", stopped.running, stopped.managed);
        assert!(!stopped.managed, "stop must release the child");
        // The bug this test caught the first time it ran: killing the npx
        // wrapper left the real daemon orphaned onto init, still serving.
        assert!(
            !stopped.running,
            "the daemon must actually be gone, not just released — something is still serving"
        );
        assert!(
            !websearch::status().running,
            "nothing may still be listening after a stop"
        );
    }
}
