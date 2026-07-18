use crate::remote::RemotePluginServiceConfig;
use codex_login::CodexAuth;
use codex_protocol::protocol::Product;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePluginMutationResponse {
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RemotePluginMutationError {
    #[error("chatgpt authentication required for remote plugin mutation")]
    AuthRequired,

    #[error(
        "chatgpt authentication required for remote plugin mutation; api key auth is not supported"
    )]
    UnsupportedAuthMode,

    #[error("failed to read auth token for remote plugin mutation: {0}")]
    AuthToken(#[source] std::io::Error),

    #[error("invalid chatgpt base url for remote plugin mutation: {0}")]
    InvalidBaseUrl(#[source] url::ParseError),

    #[error("chatgpt base url cannot be used for plugin mutation")]
    InvalidBaseUrlPath,

    #[error("failed to send remote plugin mutation request to {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("remote plugin mutation failed with status {status} from {url}: {body}")]
    UnexpectedStatus {
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("failed to parse remote plugin mutation response from {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "remote plugin mutation returned unexpected plugin id: expected `{expected}`, got `{actual}`"
    )]
    UnexpectedPluginId { expected: String, actual: String },

    #[error(
        "remote plugin mutation returned unexpected enabled state for `{plugin_id}`: expected {expected_enabled}, got {actual_enabled}"
    )]
    UnexpectedEnabledState {
        plugin_id: String,
        expected_enabled: bool,
        actual_enabled: bool,
    },

    /// kimcli-branding: returned by `post_remote_plugin_mutation` in place of ever
    /// building a request to `chatgpt_base_url` (see its doc comment below). A mutation
    /// (`enable`/`uninstall`) with a real remote side effect, so this is an honest
    /// failure rather than a fabricated success.
    #[error("kimcli does not issue plugin requests to OpenAI-hosted infrastructure (chatgpt_base_url)")]
    NetworkDisabled,
}

#[derive(Debug, thiserror::Error)]
pub enum RemotePluginFetchError {
    #[error("failed to send remote featured plugin request to {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("remote featured plugin request to {url} failed with status {status}: {body}")]
    UnexpectedStatus {
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("failed to parse remote featured plugin response from {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },
}

/// kimcli-branding override: never issues the request.
///
/// Upstream this built and sent `GET {chatgpt_base_url}/plugins/featured`
/// unconditionally -- the auth check below only gated whether auth headers were
/// *attached*, not whether the request fired at all, so this reached chatgpt.com even
/// with no ChatGPT auth present, gated only by `config.plugins_enabled` (default true)
/// via `featured_plugin_ids_for_config`, itself reachable from real app-server startup
/// (`app-server/src/message_processor.rs` -> `maybe_start_plugin_startup_tasks_for_config`).
/// This is the network-level backstop behind the policy gates
/// (`host_owned_codex_apps_enabled` / `Features::apps_enabled_for_auth` /
/// `PluginsManager::remote_global_catalog_active`, all hard-pinned false elsewhere) so
/// that this auth-independent gate-bypass path cannot reach the network either, no
/// matter what a future caller checks first. An empty Vec is `featured_plugin_ids_for_config`'s
/// existing "nothing to feature" outcome, so this is indistinguishable from a live
/// empty response to every caller.
pub async fn fetch_remote_featured_plugin_ids(
    _config: &RemotePluginServiceConfig,
    _auth: Option<&CodexAuth>,
    _product: Option<Product>,
) -> Result<Vec<String>, RemotePluginFetchError> {
    Ok(Vec::new())
}

pub async fn enable_remote_plugin(
    config: &RemotePluginServiceConfig,
    auth: Option<&CodexAuth>,
    plugin_id: &str,
) -> Result<(), RemotePluginMutationError> {
    post_remote_plugin_mutation(config, auth, plugin_id, "enable").await?;
    Ok(())
}

pub async fn uninstall_remote_plugin(
    config: &RemotePluginServiceConfig,
    auth: Option<&CodexAuth>,
    plugin_id: &str,
) -> Result<(), RemotePluginMutationError> {
    post_remote_plugin_mutation(config, auth, plugin_id, "uninstall").await?;
    Ok(())
}

// kimcli-branding: `ensure_codex_backend_auth`/`remote_plugin_mutation_url` were removed
// entirely -- once `post_remote_plugin_mutation` is pinned to never build a request
// (below), they had zero remaining callers.

/// kimcli-branding override: never issues the request. Shared by `enable_remote_plugin`
/// and `uninstall_remote_plugin` above, both of which propagate this error via `?` --
/// pinning this one function is sufficient to make both public entry points inert. This
/// is the network-level backstop behind the policy gates
/// (`host_owned_codex_apps_enabled` / `Features::apps_enabled_for_auth` /
/// `PluginsManager::remote_global_catalog_active`), same rationale as
/// `fetch_remote_featured_plugin_ids` above.
async fn post_remote_plugin_mutation(
    _config: &RemotePluginServiceConfig,
    _auth: Option<&CodexAuth>,
    _plugin_id: &str,
    _action: &str,
) -> Result<RemotePluginMutationResponse, RemotePluginMutationError> {
    Err(RemotePluginMutationError::NetworkDisabled)
}
