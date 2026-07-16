#![cfg(not(debug_assertions))]

use crate::legacy_core::config::Config;
use crate::update_versions::is_source_build_version;
use crate::updates_cache::read_version_info;
use crate::updates_cache::version_filepath;
use crate::version::CODEX_CLI_VERSION;

pub(crate) use crate::updates_cache::dismiss_version;

/// kimcli is a pinned rebrand of Codex CLI; it never checks upstream for a
/// newer release and never downloads/installs anything on the user's behalf.
/// See NOTICE and `kimcli update` for how kimcli itself is kept up to date.
pub fn get_upgrade_version(config: &Config) -> Option<String> {
    let _ = config;
    None
}

/// Returns the latest version to show in a popup, if it should be shown.
/// This respects the user's dismissal choice for the current latest version.
pub fn get_upgrade_version_for_popup(config: &Config) -> Option<String> {
    if !config.check_for_update_on_startup || is_source_build_version(CODEX_CLI_VERSION) {
        return None;
    }

    let version_file = version_filepath(config);
    let latest = get_upgrade_version(config)?;
    // If the user dismissed this exact version previously, do not show the popup.
    if let Ok(info) = read_version_info(&version_file)
        && info.dismissed_version.as_deref() == Some(latest.as_str())
    {
        return None;
    }
    Some(latest)
}
