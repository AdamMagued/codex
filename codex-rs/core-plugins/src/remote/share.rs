use super::*;
use crate::plugin_bundle_archive::PluginBundlePackError;
use crate::plugin_bundle_archive::pack_plugin_bundle_tar_gz;
use codex_login::CodexAuth;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use tracing::warn;

mod checkout;
mod local_paths;

const REMOTE_PLUGIN_SHARE_MAX_ARCHIVE_BYTES: usize = 50 * 1024 * 1024;

pub use checkout::checkout_remote_plugin_share;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePluginShareSaveResult {
    pub remote_plugin_id: String,
    pub share_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemotePluginShareAccessPolicy {
    pub discoverability: Option<RemotePluginShareDiscoverability>,
    pub share_targets: Option<Vec<RemotePluginShareTarget>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemotePluginShareDiscoverability {
    Listed,
    Unlisted,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemotePluginShareUpdateDiscoverability {
    Listed,
    Unlisted,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemotePluginSharePrincipalType {
    User,
    Group,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePluginShareTarget {
    pub principal_type: RemotePluginSharePrincipalType,
    pub principal_id: String,
    pub role: RemotePluginShareTargetRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RemotePluginSharePrincipal {
    pub principal_type: RemotePluginSharePrincipalType,
    pub principal_id: String,
    pub role: RemotePluginSharePrincipalRole,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemotePluginShareTargetRole {
    Reader,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemotePluginSharePrincipalRole {
    Reader,
    Editor,
    Owner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePluginShareUpdateTargetsResult {
    pub principals: Vec<RemotePluginSharePrincipal>,
    pub discoverability: RemotePluginShareDiscoverability,
}

// kimcli-branding: `RemoteWorkspacePluginUploadUrlRequest` was removed entirely -- it was
// only constructed as the request body inside `create_workspace_plugin_upload`, which no
// longer builds a request.

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RemoteWorkspacePluginUploadUrlResponse {
    file_id: String,
    upload_url: String,
    etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RemoteWorkspacePluginCreateRequest {
    file_id: String,
    etag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    discoverability: Option<RemotePluginShareDiscoverability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    share_targets: Option<Vec<RemotePluginShareTarget>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RemoteWorkspacePluginCreateResponse {
    plugin_id: String,
    share_url: Option<String>,
}

// kimcli-branding: `RemotePluginShareUpdateTargetsRequest`/`RemotePluginShareUpdateTargetsResponse`
// were removed entirely -- both were only used inside `update_remote_plugin_share_targets`'s
// body, which no longer builds a request.

pub async fn save_remote_plugin_share(
    config: &RemotePluginServiceConfig,
    auth: Option<&CodexAuth>,
    codex_home: &Path,
    plugin_path: &AbsolutePathBuf,
    remote_plugin_id: Option<&str>,
    access_policy: RemotePluginShareAccessPolicy,
) -> Result<RemotePluginShareSaveResult, RemotePluginCatalogError> {
    let auth = ensure_chatgpt_auth(auth)?;
    let plugin_path_for_archive = plugin_path.as_path().to_path_buf();
    let (filename, archive_bytes) = tokio::task::spawn_blocking(move || {
        let filename = archive_filename(&plugin_path_for_archive)?;
        let archive_bytes = archive_plugin_for_upload(&plugin_path_for_archive)?;
        Ok::<_, RemotePluginCatalogError>((filename, archive_bytes))
    })
    .await
    .map_err(RemotePluginCatalogError::ArchiveJoin)??;
    let upload = create_workspace_plugin_upload(
        config,
        auth,
        &filename,
        archive_bytes.len(),
        remote_plugin_id,
    )
    .await?;
    let etag = upload
        .etag
        .ok_or(RemotePluginCatalogError::MissingUploadEtag)?;
    put_workspace_plugin_upload(&upload.upload_url, archive_bytes).await?;
    let share_targets = access_policy.share_targets;
    let share_targets =
        ensure_unlisted_workspace_target(auth, access_policy.discoverability, share_targets)?;
    let response = finalize_workspace_plugin_upload(
        config,
        auth,
        remote_plugin_id,
        RemoteWorkspacePluginCreateRequest {
            file_id: upload.file_id,
            etag,
            discoverability: access_policy.discoverability,
            share_targets,
        },
    )
    .await?;
    if response.plugin_id.is_empty() {
        return Err(RemotePluginCatalogError::UnexpectedResponse(
            "workspace plugin create response did not include a plugin id".to_string(),
        ));
    }

    if let Err(err) = local_paths::record_plugin_share_local_path(
        codex_home,
        &response.plugin_id,
        plugin_path.clone(),
    ) {
        warn!(
            remote_plugin_id = %response.plugin_id,
            "failed to record plugin share local path mapping: {err}"
        );
    }

    Ok(RemotePluginShareSaveResult {
        remote_plugin_id: response.plugin_id,
        share_url: response.share_url,
    })
}

pub async fn list_remote_plugin_shares(
    config: &RemotePluginServiceConfig,
    auth: Option<&CodexAuth>,
    codex_home: &Path,
) -> Result<Vec<RemotePluginShareSummary>, RemotePluginCatalogError> {
    let auth = ensure_chatgpt_auth(auth)?;
    let created_plugins = fetch_created_workspace_plugins(config, auth).await?;
    if created_plugins.is_empty() {
        return Ok(Vec::new());
    }

    let installed_by_id =
        fetch_installed_plugins_for_scope(config, auth, RemotePluginScope::Workspace)
            .await?
            .into_iter()
            .map(|plugin| (plugin.plugin.id.clone(), plugin))
            .collect::<BTreeMap<_, _>>();
    let local_plugin_paths =
        local_paths::load_plugin_share_local_paths(codex_home).map_err(|err| {
            RemotePluginCatalogError::UnexpectedResponse(format!(
                "failed to load plugin share local path mapping: {err}"
            ))
        })?;

    created_plugins
        .into_iter()
        .map(|plugin| {
            let summary = build_remote_plugin_summary(&plugin, installed_by_id.get(&plugin.id))?;
            if summary
                .share_context
                .as_ref()
                .and_then(|context| context.share_principals.as_ref())
                .is_none()
            {
                return Err(RemotePluginCatalogError::UnexpectedResponse(format!(
                    "created workspace plugin `{}` did not include share_principals",
                    plugin.id
                )));
            }
            let local_plugin_path = local_plugin_paths.get(&plugin.id).cloned();
            Ok(RemotePluginShareSummary {
                summary,
                local_plugin_path,
            })
        })
        .collect()
}

pub fn load_plugin_share_remote_ids_by_local_path(
    codex_home: &Path,
) -> io::Result<BTreeMap<AbsolutePathBuf, String>> {
    let local_paths = local_paths::load_plugin_share_local_paths(codex_home)?;
    local_paths
        .into_iter()
        .map(|(remote_plugin_id, local_plugin_path)| {
            if !is_valid_remote_plugin_id(&remote_plugin_id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid remote plugin id in share local path mapping: {remote_plugin_id}"
                    ),
                ));
            }
            Ok((local_plugin_path, remote_plugin_id))
        })
        .collect()
}

/// kimcli-branding: never issues the request (see `fetch_recommended_plugins`'s doc
/// comment in `remote.rs` for the full rationale). A mutation with a real remote side
/// effect the local path-mapping removal depends on having actually happened, so
/// `NetworkDisabled` is returned rather than faking success and removing local state for
/// a delete that never reached the server.
pub async fn delete_remote_plugin_share(
    _config: &RemotePluginServiceConfig,
    _auth: Option<&CodexAuth>,
    _codex_home: &Path,
    _remote_plugin_id: &str,
) -> Result<(), RemotePluginCatalogError> {
    Err(RemotePluginCatalogError::NetworkDisabled)
}

/// kimcli-branding: never issues the request (see `fetch_recommended_plugins`'s doc
/// comment in `remote.rs` for the full rationale).
pub async fn update_remote_plugin_share_targets(
    _config: &RemotePluginServiceConfig,
    _auth: Option<&CodexAuth>,
    _remote_plugin_id: &str,
    _targets: Vec<RemotePluginShareTarget>,
    _discoverability: RemotePluginShareUpdateDiscoverability,
) -> Result<RemotePluginShareUpdateTargetsResult, RemotePluginCatalogError> {
    Err(RemotePluginCatalogError::NetworkDisabled)
}

fn ensure_unlisted_workspace_target(
    auth: &CodexAuth,
    discoverability: Option<RemotePluginShareDiscoverability>,
    targets: Option<Vec<RemotePluginShareTarget>>,
) -> Result<Option<Vec<RemotePluginShareTarget>>, RemotePluginCatalogError> {
    if discoverability != Some(RemotePluginShareDiscoverability::Unlisted) {
        return Ok(targets);
    }
    let account_id = auth.get_account_id().ok_or_else(|| {
        RemotePluginCatalogError::UnexpectedResponse(
            "workspace plugin share requires an account id".to_string(),
        )
    })?;
    let mut targets = targets.unwrap_or_default();
    if !targets.iter().any(|target| {
        target.principal_type == RemotePluginSharePrincipalType::Workspace
            && target.principal_id == account_id
    }) {
        targets.push(RemotePluginShareTarget {
            principal_type: RemotePluginSharePrincipalType::Workspace,
            principal_id: account_id,
            role: RemotePluginShareTargetRole::Reader,
        });
    }
    Ok(Some(targets))
}

async fn fetch_created_workspace_plugins(
    config: &RemotePluginServiceConfig,
    auth: &CodexAuth,
) -> Result<Vec<RemotePluginDirectoryItem>, RemotePluginCatalogError> {
    let mut plugins = Vec::new();
    let mut page_token = None;
    loop {
        let response =
            get_created_workspace_plugins_page(config, auth, page_token.as_deref()).await?;
        plugins.extend(response.plugins);
        let Some(next_page_token) = response.pagination.next_page_token else {
            break;
        };
        page_token = Some(next_page_token);
    }
    Ok(plugins)
}

/// kimcli-branding: never issues the request (see `fetch_recommended_plugins`'s doc
/// comment in `remote.rs` for the full rationale). Empty page, no next-page-token --
/// `list_remote_plugin_shares` already treats an empty page as "no created workspace
/// plugins" and returns `Ok(Vec::new())` gracefully.
async fn get_created_workspace_plugins_page(
    _config: &RemotePluginServiceConfig,
    _auth: &CodexAuth,
    _page_token: Option<&str>,
) -> Result<RemotePluginListResponse, RemotePluginCatalogError> {
    Ok(RemotePluginListResponse {
        plugins: Vec::new(),
        pagination: RemotePluginPagination {
            next_page_token: None,
        },
    })
}

/// kimcli-branding: never issues the request (see `fetch_recommended_plugins`'s doc
/// comment in `remote.rs` for the full rationale). `save_remote_plugin_share`'s only
/// caller of this function propagates the error via `?` before ever reaching
/// `put_workspace_plugin_upload`, so pinning this one function is sufficient to make
/// the whole share-upload flow inert.
async fn create_workspace_plugin_upload(
    _config: &RemotePluginServiceConfig,
    _auth: &CodexAuth,
    _filename: &str,
    _size_bytes: usize,
    _remote_plugin_id: Option<&str>,
) -> Result<RemoteWorkspacePluginUploadUrlResponse, RemotePluginCatalogError> {
    Err(RemotePluginCatalogError::NetworkDisabled)
}

/// kimcli-branding: never issues the request. Upstream this PUTs the plugin archive
/// bytes to `upload_url` (an arbitrary, server-issued URL rather than `chatgpt_base_url`
/// itself). `save_remote_plugin_share`'s only caller already fails earlier at
/// `create_workspace_plugin_upload` (which is what would have supplied a real
/// `upload_url`), but that is exactly the "trust the caller" reasoning this commit
/// exists to eliminate -- this function still built and sent a real request on its own,
/// so it is pinned independently, at its own top, like every other request-builder in
/// this module.
async fn put_workspace_plugin_upload(
    _upload_url: &str,
    _archive_bytes: Vec<u8>,
) -> Result<(), RemotePluginCatalogError> {
    Err(RemotePluginCatalogError::NetworkDisabled)
}

/// kimcli-branding: never issues the request (see `fetch_recommended_plugins`'s doc
/// comment in `remote.rs` for the full rationale). `save_remote_plugin_share` never
/// reaches this call in practice since `create_workspace_plugin_upload` above already
/// errors first, but this is pinned independently too, per-function, rather than
/// relying on that upstream short-circuit.
async fn finalize_workspace_plugin_upload(
    _config: &RemotePluginServiceConfig,
    _auth: &CodexAuth,
    _remote_plugin_id: Option<&str>,
    _body: RemoteWorkspacePluginCreateRequest,
) -> Result<RemoteWorkspacePluginCreateResponse, RemotePluginCatalogError> {
    Err(RemotePluginCatalogError::NetworkDisabled)
}

fn archive_filename(plugin_path: &Path) -> Result<String, RemotePluginCatalogError> {
    let plugin_name = plugin_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RemotePluginCatalogError::InvalidPluginPath {
            path: plugin_path.to_path_buf(),
            reason: "plugin path must end in a valid UTF-8 directory name".to_string(),
        })?;
    Ok(format!("{plugin_name}.tar.gz"))
}

fn archive_plugin_for_upload(plugin_path: &Path) -> Result<Vec<u8>, RemotePluginCatalogError> {
    archive_plugin_for_upload_with_limit(plugin_path, REMOTE_PLUGIN_SHARE_MAX_ARCHIVE_BYTES)
}

fn archive_plugin_for_upload_with_limit(
    plugin_path: &Path,
    max_bytes: usize,
) -> Result<Vec<u8>, RemotePluginCatalogError> {
    pack_plugin_bundle_tar_gz(plugin_path, max_bytes).map_err(|err| match err {
        PluginBundlePackError::InvalidPluginPath { path, reason } => {
            RemotePluginCatalogError::InvalidPluginPath { path, reason }
        }
        PluginBundlePackError::ArchiveTooLarge { bytes, max_bytes } => {
            RemotePluginCatalogError::ArchiveTooLarge { bytes, max_bytes }
        }
        PluginBundlePackError::Io { source } => RemotePluginCatalogError::Archive {
            path: plugin_path.to_path_buf(),
            source,
        },
    })
}

// kimcli-branding: `send_and_expect_status` (the shared request-sending helper
// `delete_remote_plugin_share` used to call) was removed entirely -- once that function
// is pinned to never build a request (see its doc comment above), this had zero
// remaining callers. Mirrors the same removal of `remote.rs`'s `send_and_decode`/
// `authenticated_request`.

#[cfg(test)]
mod tests;
