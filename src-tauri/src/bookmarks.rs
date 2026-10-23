//! Security-scoped bookmarks — the token a sandboxed macOS app needs to reopen
//! a folder the user once chose.
//!
//! `recent-projects.json` used to store plain paths. A path is only text: under
//! the App Sandbox the folder behind it is unreadable on the next launch, so
//! every recent project would fail to open in a signed build even though the
//! list looked healthy. A bookmark is the token the sandbox honours.
//!
//! Two honest limits, both handled here rather than hidden:
//!
//! - Creating a *security-scoped* bookmark needs the
//!   `com.apple.security.files.user-selected.read-write` entitlement. A plain
//!   `cargo test` or `tauri dev` binary has no sandbox and no entitlement, so
//!   that call fails; a plain bookmark is made instead. It still round-trips,
//!   and it still follows a folder that has been moved or renamed.
//! - Everything here is macOS. On other platforms the path already works, so
//!   the functions return `None` and the caller falls back to the stored path.
//!
//! A `None` from this module is therefore never an error to report — it means
//! "use the path", which is correct off the sandbox.

use std::path::{Path, PathBuf};

/// Bookmark bytes as lowercase hex. Hex rather than base64 keeps this module
/// dependency-free; a bookmark is a couple of kilobytes, so the doubling costs
/// nothing a human would notice in a twelve-entry JSON file.
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        out.push(char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
    }
    out
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.is_empty() || text.len() % 2 != 0 {
        return None;
    }
    let raw: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(raw.len() / 2);
    for pair in raw.chunks(2) {
        let hi = pair[0].to_digit(16)?;
        let lo = pair[1].to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{from_hex, to_hex};
    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2_foundation::{
        NSData, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions,
    };
    use std::path::{Path, PathBuf};

    fn url_for(path: &Path) -> Option<Retained<NSURL>> {
        let text = NSString::from_str(path.to_str()?);
        Some(NSURL::fileURLWithPath(&text))
    }

    pub fn create(path: &Path) -> Option<String> {
        let url = url_for(path)?;
        // Security scope first: that is what a sandboxed build needs. Without
        // the entitlement it fails, and a plain bookmark is the honest
        // fallback — it reopens a moved folder, it just carries no sandbox
        // grant, which an unsandboxed build does not need.
        for options in [
            NSURLBookmarkCreationOptions::WithSecurityScope,
            NSURLBookmarkCreationOptions::empty(),
        ] {
            if let Ok(data) =
                url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                    options, None, None,
                )
            {
                return Some(to_hex(&data.to_vec()));
            }
        }
        None
    }

    pub fn resolve(token: &str) -> Option<PathBuf> {
        let bytes = from_hex(token)?;
        let data = NSData::with_bytes(&bytes);
        for options in [
            NSURLBookmarkResolutionOptions::WithSecurityScope,
            NSURLBookmarkResolutionOptions::empty(),
        ] {
            let mut stale = Bool::NO;
            let resolved = unsafe {
                NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                    &data,
                    options,
                    None,
                    &mut stale as *mut Bool,
                )
            };
            // A stale bookmark still resolves; the caller rewrites a fresh one
            // the next time the project is opened, so staleness is not fatal.
            if let Ok(url) = resolved {
                if let Some(text) = url.path() {
                    return Some(PathBuf::from(text.to_string()));
                }
            }
        }
        None
    }

    pub fn hold(token: &str) -> Option<PathBuf> {
        let bytes = from_hex(token)?;
        let data = NSData::with_bytes(&bytes);
        let mut stale = Bool::NO;
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                NSURLBookmarkResolutionOptions::WithSecurityScope,
                None,
                &mut stale as *mut Bool,
            )
        }
        .ok()?;
        let path = url.path().map(|t| PathBuf::from(t.to_string()))?;
        if unsafe { url.startAccessingSecurityScopedResource() } {
            // Deliberately never stopped. The project stays open until the app
            // quits, and revoking access mid-save would corrupt a write that is
            // already in flight; the kernel reclaims the grant on exit.
            std::mem::forget(url);
            Some(path)
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::path::{Path, PathBuf};

    pub fn create(_path: &Path) -> Option<String> {
        None
    }
    pub fn resolve(_token: &str) -> Option<PathBuf> {
        None
    }
    pub fn hold(_token: &str) -> Option<PathBuf> {
        None
    }
}

/// A bookmark for `path`, or `None` when the platform has no such concept.
pub fn create(path: &Path) -> Option<String> {
    imp::create(path)
}

/// The folder a bookmark points at now — which is not always where it pointed
/// when it was made, since a bookmark follows a rename or a move.
pub fn resolve(token: &str) -> Option<PathBuf> {
    imp::resolve(token)
}

/// Resolve and claim sandbox access for the life of the process.
pub fn hold(token: &str) -> Option<PathBuf> {
    imp::hold(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_every_byte() {
        let all: Vec<u8> = (0..=255u8).collect();
        assert_eq!(from_hex(&to_hex(&all)), Some(all));
        assert_eq!(from_hex("odd"), None);
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex(""), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_bookmark_round_trips_to_bytes_and_back_to_the_same_folder() {
        let dir = std::env::temp_dir().join(format!("aegis-bookmark-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let token = create(&dir).expect("macOS should always make at least a plain bookmark");
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));

        let back = resolve(&token).expect("a bookmark this process just made must resolve");
        // /tmp is a symlink to /private/tmp, and a bookmark resolves to the
        // real folder, so compare canonical paths rather than the strings.
        assert_eq!(
            std::fs::canonicalize(&back).unwrap(),
            std::fs::canonicalize(&dir).unwrap()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_bookmark_follows_a_folder_that_moved() {
        let base = std::env::temp_dir().join(format!("aegis-moved-{}", std::process::id()));
        let before = base.join("before");
        let after = base.join("after");
        std::fs::create_dir_all(&before).unwrap();

        let token = create(&before).unwrap();
        std::fs::rename(&before, &after).unwrap();

        let back = resolve(&token).expect("a bookmark tracks the folder, not the path");
        assert_eq!(
            std::fs::canonicalize(&back).unwrap(),
            std::fs::canonicalize(&after).unwrap()
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn nonsense_resolves_to_nothing_rather_than_panicking() {
        assert_eq!(resolve("not hex at all"), None);
        assert_eq!(resolve("deadbeef"), None);
    }
}
