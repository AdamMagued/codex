/// kimcli does not bundle, download, or install a desktop application, and never
/// launches one on the user's behalf.
///
/// Upstream, this subcommand opens (or silently downloads and installs, from
/// `persistent.oaistatic.com` on macOS or the Microsoft Store on Windows) OpenAI's
/// Codex Desktop app. kimcli must never install or advertise OpenAI's software: this
/// is the same standing product constraint as the disabled self-update path (see
/// `run_update_command` in `main.rs`), applied here because Kim ships its own,
/// separate desktop app (the kim-pro project) that this CLI has no business
/// downloading, installing, or opening in place of. So `kimcli app` always refuses
/// with a non-zero exit, on every platform, rather than gate the installer logic
/// behind a config flag or platform check. The former `desktop_app` module (the
/// mac/Windows installer/open implementations and their oaistatic.com/Microsoft
/// Store URLs) has been removed entirely since it has no remaining caller.
pub async fn run_app() -> anyhow::Result<()> {
    anyhow::bail!(
        "kimcli does not bundle or install a desktop app, and will not download or launch \
         OpenAI's Codex Desktop on your behalf. Kim has its own, separate desktop app — install \
         it from the kim-pro project instead of running `kimcli app`."
    );
}
