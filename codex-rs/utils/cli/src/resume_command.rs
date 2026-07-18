//! Shared formatting for user-facing `kimcli resume` command hints.
//!
//! kimcli-branding: this is the single source of the "To continue this session, run ..." exit
//! hint and its siblings shown throughout codex-rs/tui (main.rs's exit message,
//! app/session_lifecycle.rs, app/event_dispatch.rs, chatwidget.rs's thread-rename hint). It used
//! to hardcode "codex resume ...", which told the user to run a command that does not exist in
//! this fork -- the installed binary is `kimcli` (see cli/Cargo.toml's `[[bin]] name = "kimcli"`;
//! there is no `codex` binary), and `kimcli resume <target>` is the real, working equivalent
//! (same clap parser, just built under the `kimcli` binary name). Rebranded so the hint actually
//! works when copy-pasted.

use codex_protocol::ThreadId;
use codex_shell_command::parse_command::shlex_join;

pub fn resume_command(thread_name: Option<&str>, thread_id: Option<ThreadId>) -> Option<String> {
    let resume_target = thread_name
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| thread_id.map(|thread_id| thread_id.to_string()));
    resume_target.map(|target| {
        let needs_double_dash = target.starts_with('-');
        let escaped = shlex_join(&[target]);
        if needs_double_dash {
            format!("kimcli resume -- {escaped}")
        } else {
            format!("kimcli resume {escaped}")
        }
    })
}

pub fn resume_hint(thread_name: Option<&str>, thread_id: Option<ThreadId>) -> Option<String> {
    let thread_id = thread_id?;
    match thread_name.filter(|name| !name.is_empty()) {
        Some(thread_name) => Some(format!(
            "kimcli resume, then select {thread_name} ({thread_id})"
        )),
        None => resume_command(/*thread_name*/ None, Some(thread_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn prefers_name_over_id() {
        let thread_id = ThreadId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let command = resume_command(Some("my-thread"), Some(thread_id));
        assert_eq!(command, Some("kimcli resume my-thread".to_string()));
    }

    #[test]
    fn formats_thread_id_when_name_is_missing() {
        let thread_id = ThreadId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let command = resume_command(/*thread_name*/ None, Some(thread_id));
        assert_eq!(
            command,
            Some("kimcli resume 123e4567-e89b-12d3-a456-426614174000".to_string())
        );
    }

    #[test]
    fn returns_none_without_a_resume_target() {
        let command = resume_command(/*thread_name*/ None, /*thread_id*/ None);
        assert_eq!(command, None);
    }

    #[test]
    fn quotes_thread_names_when_needed() {
        let command = resume_command(Some("-starts-with-dash"), /*thread_id*/ None);
        assert_eq!(
            command,
            Some("kimcli resume -- -starts-with-dash".to_string())
        );

        let command = resume_command(Some("two words"), /*thread_id*/ None);
        assert_eq!(command, Some("kimcli resume 'two words'".to_string()));

        let command = resume_command(Some("quote'case"), /*thread_id*/ None);
        assert_eq!(command, Some("kimcli resume \"quote'case\"".to_string()));
    }

    #[test]
    fn resume_hint_names_picker_item_with_id() {
        let thread_id = ThreadId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let hint = resume_hint(Some("my-thread"), Some(thread_id));
        assert_eq!(
            hint,
            Some(
                "kimcli resume, then select my-thread (123e4567-e89b-12d3-a456-426614174000)"
                    .to_string()
            )
        );
    }

    #[test]
    fn resume_hint_uses_direct_id_command_without_name() {
        let thread_id = ThreadId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let hint = resume_hint(/*thread_name*/ None, Some(thread_id));
        assert_eq!(
            hint,
            Some("kimcli resume 123e4567-e89b-12d3-a456-426614174000".to_string())
        );
    }

    #[test]
    fn resume_hint_requires_thread_id() {
        let hint = resume_hint(Some("my-thread"), /*thread_id*/ None);
        assert_eq!(hint, None);
    }
}
