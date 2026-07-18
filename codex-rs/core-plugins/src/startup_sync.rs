use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use reqwest::Client;
use tracing::warn;

// kimcli-branding: the `http_client` submodule (the request-builder infra) is kept because the
// startup-sync unit tests still construct a `StartupSyncHttpClient`. It builds request builders
// but issues nothing on its own; every production caller in this file short-circuits before
// reaching it (see `CURATED_PLUGINS_SYNC_NETWORK_DISABLED`), so no auto-fire path survives.
mod http_client;
use self::http_client::StartupSyncHttpClient;
use self::http_client::StartupSyncRequestBuilder;
use codex_login::default_client::default_headers;
use http::Method;

// kimcli-branding: `build_reqwest_client` and the GitHub/backup-archive JSON response structs
// (`GitHubRepositorySummary`, `GitHubGitRefSummary`, `GitHubGitRefObject`,
// `CuratedPluginsBackupArchiveResponse`) were removed entirely -- every function that used them
// is pinned below to return before ever constructing a client or parsing a response. See
// CURATED_PLUGINS_SYNC_NETWORK_DISABLED.

// kimcli-branding: this module is upstream's "curated plugins" auto-fire startup sync -- on
// every launch (see manager.rs's `start_curated_repo_sync`), unauthenticated, with no opt-in,
// it used to `git ls-remote`/`git fetch` `https://github.com/openai/plugins.git`, fall back to
// `https://api.github.com` REST calls, and fall back again to
// `https://chatgpt.com/backend-api/plugins/export/curated`. That is exactly the class of
// OpenAI-phoning behavior this whole rebrand exists to eliminate, and it is worse than the
// plugin-marketplace paths pinned earlier because it fires on startup with no auth and no user
// action at all. Every function in this file that would clone/fetch `openai/plugins.git` or
// send an HTTP request to api.github.com/chatgpt.com is pinned below to return this error (or,
// for the repo-sync orchestrators, an empty/"nothing synced" success) before doing any of that
// work -- independently of its caller, matching the same principle applied to the plugin
// marketplace network pin. See also manager.rs's `start_curated_repo_sync`, which no longer
// spawns the sync thread at all.
const CURATED_PLUGINS_SYNC_NETWORK_DISABLED: &str = "kimcli does not sync curated plugins from \
    openai/plugins.git, api.github.com, or chatgpt.com (curated plugin sync is disabled)";

// kimcli-branding: kept only because `#[ignore]`d tests in startup_sync_tests.rs still reference
// it directly (see `sync_openai_plugins_repo_via_git_succeeds_with_local_rewritten_remote`);
// every real caller was pinned above.
#[allow(dead_code)]
const CURATED_PLUGINS_FETCH_REF: &str = "refs/codex/curated-sync";
const CURATED_PLUGINS_RELATIVE_DIR: &str = ".tmp/plugins";
const CURATED_PLUGINS_SHA_FILE: &str = ".tmp/plugins.sha";
// kimcli-branding: kept only for `remove_stale_curated_repo_temp_dirs`'s own unit test (its
// production caller, `prepare_curated_repo_parent_and_temp_dir`, was deleted as dead code once
// every sync entry point that used it was pinned).
#[allow(dead_code)]
// Keep this comfortably above a normal sync attempt so we do not race another Codex process.
const CURATED_PLUGINS_STALE_TEMP_DIR_MAX_AGE: Duration = Duration::from_secs(10 * 60);
// These variables can redirect Git away from the repository selected by `-C`,
// or inject command-scoped configuration into the sync commands.
const REPOSITORY_LOCAL_GIT_ENVIRONMENT_VARIABLES: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_CONFIG",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_PARAMETERS",
    "GIT_DIR",
    "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    "GIT_GRAFT_FILE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_NAMESPACE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_REPLACE_REF_BASE",
    "GIT_SHALLOW_FILE",
    "GIT_WORK_TREE",
];

pub fn curated_plugins_repo_path(codex_home: &Path) -> PathBuf {
    codex_home.join(CURATED_PLUGINS_RELATIVE_DIR)
}

pub fn curated_plugins_api_marketplace_path(codex_home: &Path) -> PathBuf {
    curated_plugins_repo_path(codex_home).join(".agents/plugins/api_marketplace.json")
}

pub fn read_curated_plugins_sha(codex_home: &Path) -> Option<String> {
    read_sha_file(curated_plugins_sha_path(codex_home).as_path())
}

fn curated_plugins_sha_path(codex_home: &Path) -> PathBuf {
    codex_home.join(CURATED_PLUGINS_SHA_FILE)
}

/// kimcli-branding: the public entry point for the curated-plugins startup sync. Upstream this
/// resolved a git binary and delegated to `sync_openai_plugins_repo_with_transport_overrides`,
/// which chains git -> GitHub REST -> chatgpt.com export-archive fallbacks, each capable of
/// issuing a real network request. It never checks auth or any opt-in flag. This function's own
/// production caller, manager.rs's `start_curated_repo_sync`, no longer even invokes it (see
/// that function's doc comment) -- but this pin exists independently, so no future caller of
/// `sync_openai_plugins_repo` itself can trigger a clone or HTTP request either. Returns the
/// same `Ok(String)` shape upstream uses for "nothing to sync / already current" (an empty sha
/// is never treated as an error by this function's own callers), not an `Err` that would spam
/// warnings on every kimcli launch.
pub fn sync_openai_plugins_repo(_codex_home: &Path) -> Result<String, String> {
    Ok(String::new())
}

/// kimcli-branding: pinned along with every other function in this file that could reach
/// `openai/plugins.git`, `api.github.com`, or `chatgpt.com` -- see
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Currently unreachable in production (its only
/// production caller, `sync_openai_plugins_repo`, is pinned above and no longer calls it), but
/// kept and independently pinned -- rather than deleted -- as a standing guard in case a future
/// change reconnects a caller directly to this transport-fallback orchestrator. Still exercised
/// directly by tests in startup_sync_tests.rs (marked `#[ignore]`, see this commit).
#[allow(dead_code)]
fn sync_openai_plugins_repo_with_transport_overrides(
    _codex_home: &Path,
    _git_binary: Option<&Path>,
    _api_base_url: &str,
    _backup_archive_api_url: &str,
) -> Result<String, String> {
    Ok(String::new())
}

/// kimcli-branding: the git-clone/fetch transport for the curated plugins sync. Upstream this
/// ran `git ls-remote`/`git fetch` against `https://github.com/openai/plugins.git`. See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept (not deleted) as a standing guard and because it
/// is still exercised directly by `#[ignore]`d tests in startup_sync_tests.rs.
#[allow(dead_code)]
fn sync_openai_plugins_repo_via_git(
    _codex_home: &Path,
    _git_binary: &Path,
) -> Result<String, String> {
    Ok(String::new())
}

/// kimcli-branding: `git fetch`es `https://github.com/openai/plugins.git` directly. Pinned
/// independently of its only real caller, `sync_openai_plugins_repo_via_git` (itself also pinned
/// above), matching the "pin the leaf, don't trust the caller" principle used throughout this
/// rebrand -- see CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it
/// currently has no live caller.
#[allow(dead_code)]
fn fetch_curated_plugins_commit(
    _repo_path: &Path,
    _remote_sha: &str,
    _git_binary: &Path,
) -> Result<(), String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: the shared `git fetch <source> <refspec>` executor. `source` is a generic
/// `&Path` that upstream's own `fetch_curated_plugins_commit` populated with
/// `https://github.com/openai/plugins.git` -- nothing in this function's own signature prevents
/// a future caller from doing the same, so it is pinned here directly rather than trusting that
/// only local sources are ever passed. See CURATED_PLUGINS_SYNC_NETWORK_DISABLED.
#[allow(dead_code)]
fn fetch_curated_plugins_commit_from(
    _repo_path: &Path,
    _source: &Path,
    _source_revision: &str,
    _git_binary: &Path,
    _context: &str,
) -> Result<(), String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: the GitHub REST API transport for the curated plugins sync (the fallback
/// used when git is unavailable). Upstream this called `api.github.com` via
/// `fetch_curated_repo_remote_sha`/`fetch_curated_repo_zipball`. See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard; still exercised directly by
/// `#[ignore]`d tests in startup_sync_tests.rs.
#[allow(dead_code)]
fn sync_openai_plugins_repo_via_http(
    _codex_home: &Path,
    _api_base_url: &str,
) -> Result<String, String> {
    Ok(String::new())
}

/// kimcli-branding: the `chatgpt.com/backend-api/plugins/export/curated` fallback transport --
/// the last resort when both git and the GitHub REST API fail. See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard; still exercised directly by
/// `#[ignore]`d tests in startup_sync_tests.rs.
#[allow(dead_code)]
fn sync_openai_plugins_repo_via_backup_archive(
    _codex_home: &Path,
    _backup_archive_api_url: &str,
) -> Result<String, String> {
    Ok(String::new())
}

pub fn has_local_curated_plugins_snapshot(codex_home: &Path) -> bool {
    curated_plugins_repo_path(codex_home)
        .join(".agents/plugins/marketplace.json")
        .is_file()
        && codex_home.join(CURATED_PLUGINS_SHA_FILE).is_file()
}

// kimcli-branding: `prepare_curated_repo_parent_and_temp_dir` (the only production caller) was
// deleted as dead code once every curated-plugins sync entry point that used it was pinned (see
// CURATED_PLUGINS_SYNC_NETWORK_DISABLED). This function itself does no networking -- it just
// removes stale local temp directories -- so it is kept, with `#[allow(dead_code)]`, purely
// because it still has its own meaningful unit test in startup_sync_tests.rs.
#[allow(dead_code)]
fn remove_stale_curated_repo_temp_dirs(parent: &Path, max_age: Duration) {
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(
                error = %err,
                parent = %parent.display(),
                "failed to list curated plugins temp directory parent for stale cleanup"
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                warn!(
                    error = %err,
                    path = %entry.path().display(),
                    "failed to inspect curated plugins temp directory entry"
                );
                continue;
            }
        };
        if !file_type.is_dir() {
            continue;
        }

        let path = entry.path();
        let is_plugins_clone_dir = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("plugins-clone-"));
        if !is_plugins_clone_dir {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!(
                    error = %err,
                    path = %path.display(),
                    "failed to read curated plugins temp directory metadata"
                );
                continue;
            }
        };
        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(err) => {
                warn!(
                    error = %err,
                    path = %path.display(),
                    "failed to read curated plugins temp directory modification time"
                );
                continue;
            }
        };
        let age = match modified.elapsed() {
            Ok(age) => age,
            Err(err) => {
                warn!(
                    error = %err,
                    path = %path.display(),
                    "failed to compute curated plugins temp directory age"
                );
                continue;
            }
        };
        if age < max_age {
            continue;
        }

        if let Err(err) = std::fs::remove_dir_all(&path) {
            warn!(
                error = %err,
                path = %path.display(),
                "failed to remove stale curated plugins temp directory"
            );
        }
    }
}

// kimcli-branding: `ensure_marketplace_manifest_exists`, `activate_curated_repo`,
// `write_curated_plugins_sha`, `read_local_git_or_sha_file`, `git_head_sha`,
// `run_git_command_with_timeout`, and `ensure_git_success` were removed entirely -- they were
// pure plumbing for the git/HTTP/backup-archive sync transports above, had zero remaining
// callers once those transports were pinned, and (unlike `remove_stale_curated_repo_temp_dirs`
// or `read_extracted_backup_archive_git_sha` below) had no independent unit test coverage of
// their own to preserve. The 3 `emit_curated_plugins_startup_sync_*` telemetry helpers were
// removed for the same reason (their only callers were inside
// `sync_openai_plugins_repo_with_transport_overrides`'s deleted body); permanently-disabled
// startup sync has nothing to emit telemetry about.

/// kimcli-branding: hardcodes `git ls-remote https://github.com/openai/plugins.git HEAD`. See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it currently has
/// no live caller (its only production caller, `sync_openai_plugins_repo_via_git`, is pinned
/// above).
#[allow(dead_code)]
fn git_ls_remote_head_sha(_git_binary: &Path) -> Result<String, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

// kimcli-branding: kept only for its own unit test in startup_sync_tests.rs
// (`git_command_sanitizes_ambient_repository_environment`) -- its only production caller,
// `git_ls_remote_head_sha`, is pinned above and no longer calls it. This function itself does no
// networking; it just builds a `Command` with a sanitized environment.
#[allow(dead_code)]
fn git_command(git_binary: &Path) -> Command {
    let mut command = Command::new(git_binary);
    command.env("GIT_OPTIONAL_LOCKS", "0");
    for name in REPOSITORY_LOCAL_GIT_ENVIRONMENT_VARIABLES {
        command.env_remove(name);
    }
    command
}

// kimcli-branding: kept only for its own unit test in startup_sync_tests.rs
// (`apple_git_without_developer_tools_is_unavailable`) -- its only production caller was inside
// `sync_openai_plugins_repo`'s deleted macOS branch, above.
#[allow(dead_code)]
#[cfg(any(target_os = "macos", test))]
fn macos_git_binary_from_path(
    git_path: PathBuf,
    apple_developer_tools_available: bool,
) -> Option<PathBuf> {
    if git_path == Path::new("/usr/bin/git") && !apple_developer_tools_available {
        None
    } else {
        Some(git_path)
    }
}

// kimcli-branding: `run_git_command_with_timeout` and `ensure_git_success` were removed
// entirely -- they were the shared subprocess-running/exit-status-checking plumbing behind
// every pinned git function above, had zero remaining callers, and had no independent unit test
// coverage of their own.

/// kimcli-branding: builds `client` via `build_reqwest_client()` and would `GET
/// {api_base_url}/repos/openai/plugins` and `.../git/ref/heads/<branch>` from
/// `api.github.com`. See CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even
/// though it currently has no live caller (its only production caller,
/// `sync_openai_plugins_repo_via_http`, is pinned above).
#[allow(dead_code)]
async fn fetch_curated_repo_remote_sha(_api_base_url: &str) -> Result<String, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: builds `client` via `build_reqwest_client()` and would `GET` a zipball from
/// `api.github.com`. See CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even
/// though it currently has no live caller (its only production caller,
/// `sync_openai_plugins_repo_via_http`, is pinned above).
#[allow(dead_code)]
async fn fetch_curated_repo_zipball(
    _api_base_url: &str,
    _remote_sha: &str,
) -> Result<Vec<u8>, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: builds `client` via `build_reqwest_client()` and would `GET
/// backup_archive_api_url` (`https://chatgpt.com/backend-api/plugins/export/curated`) and then
/// the `download_url` it returns. See CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing
/// guard even though it currently has no live caller (its only production caller,
/// `sync_openai_plugins_repo_via_backup_archive`, is pinned above).
#[allow(dead_code)]
async fn fetch_curated_repo_backup_archive_zip(
    _backup_archive_api_url: &str,
) -> Result<Vec<u8>, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

fn read_extracted_backup_archive_git_sha(repo_path: &Path) -> Result<Option<String>, String> {
    let git_dir = repo_path.join(".git");
    if !git_dir.is_dir() {
        return Ok(None);
    }

    let head_path = git_dir.join("HEAD");
    let head = std::fs::read_to_string(&head_path).map_err(|err| {
        format!(
            "failed to read curated plugins backup archive git HEAD {}: {err}",
            head_path.display()
        )
    })?;
    let head = head.trim();
    if head.is_empty() {
        return Err(format!(
            "curated plugins backup archive git HEAD is empty at {}",
            head_path.display()
        ));
    }

    if let Some(reference) = head.strip_prefix("ref: ") {
        let reference = validate_backup_archive_git_ref(reference.trim())?;
        return read_git_ref_sha(&git_dir, reference).map(Some);
    }

    Ok(Some(head.to_string()))
}

fn validate_backup_archive_git_ref(reference: &str) -> Result<&str, String> {
    if !reference.starts_with("refs/") {
        return Err(format!(
            "curated plugins backup archive git ref must stay under refs/: {reference}"
        ));
    }

    let path = Path::new(reference);
    if path.is_absolute() {
        return Err(format!(
            "curated plugins backup archive git ref must be relative: {reference}"
        ));
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(format!(
                    "curated plugins backup archive git ref contains invalid path components: {reference}"
                ));
            }
        }
    }

    Ok(reference)
}

fn read_git_ref_sha(git_dir: &Path, reference: &str) -> Result<String, String> {
    let ref_path = git_dir.join(reference);
    if let Ok(sha) = std::fs::read_to_string(&ref_path) {
        let sha = sha.trim();
        if sha.is_empty() {
            return Err(format!(
                "curated plugins backup archive git ref {reference} is empty at {}",
                ref_path.display()
            ));
        }
        return Ok(sha.to_string());
    }

    let packed_refs_path = git_dir.join("packed-refs");
    if let Ok(packed_refs) = std::fs::read_to_string(&packed_refs_path)
        && let Some(sha) = packed_refs.lines().find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('^') {
                return None;
            }
            let (sha, candidate_ref) = trimmed.split_once(' ')?;
            (candidate_ref == reference).then_some(sha.to_string())
        })
    {
        return Ok(sha);
    }

    Err(format!(
        "failed to resolve curated plugins backup archive git ref {reference} from {}",
        git_dir.display()
    ))
}

/// kimcli-branding: the literal `.send()` leaf for GitHub API JSON responses. See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it currently has
/// no live caller (its callers, `fetch_curated_repo_remote_sha`, are pinned above).
#[allow(dead_code)]
async fn fetch_github_text(_client: &Client, _url: &str, _context: &str) -> Result<String, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: the literal `.send()` leaf for GitHub API binary responses (zipballs). See
/// CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it currently has
/// no live caller (its caller, `fetch_curated_repo_zipball`, is pinned above).
#[allow(dead_code)]
async fn fetch_github_bytes(
    _client: &Client,
    _url: &str,
    _context: &str,
) -> Result<Vec<u8>, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: the literal `.send()` leaf for the chatgpt.com export-archive metadata
/// response. See CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it
/// currently has no live caller (its caller, `fetch_curated_repo_backup_archive_zip`, is pinned
/// above).
#[allow(dead_code)]
async fn fetch_public_text(_client: &Client, _url: &str, _context: &str) -> Result<String, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

/// kimcli-branding: the literal `.send()` leaf for the chatgpt.com export-archive zip download.
/// See CURATED_PLUGINS_SYNC_NETWORK_DISABLED. Kept as a standing guard even though it currently
/// has no live caller (its caller, `fetch_curated_repo_backup_archive_zip`, is pinned above).
#[allow(dead_code)]
async fn fetch_public_bytes(
    _client: &Client,
    _url: &str,
    _context: &str,
) -> Result<Vec<u8>, String> {
    Err(CURATED_PLUGINS_SYNC_NETWORK_DISABLED.to_string())
}

// kimcli-branding: `github_request`, `extract_zipball_to_dir`, and `apply_zip_permissions` were
// removed entirely -- they were request-building/archive-extraction plumbing with zero
// remaining callers once the functions above were pinned, and had no independent unit test
// coverage of their own.

#[allow(dead_code)]
fn startup_sync_request(
    http_clients: &StartupSyncHttpClient,
    url: &str,
) -> StartupSyncRequestBuilder {
    http_clients
        .request(Method::GET, url)
        .headers(default_headers())
}

fn read_sha_file(sha_path: &Path) -> Option<String> {
    std::fs::read_to_string(sha_path)
        .ok()
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
}

#[cfg(test)]
#[path = "startup_sync_tests.rs"]
mod tests;
