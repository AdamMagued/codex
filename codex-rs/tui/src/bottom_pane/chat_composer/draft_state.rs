//! Editable composer draft state kept separate from composer control flow.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::bottom_pane::MentionBinding;
use crate::bottom_pane::paste_burst::PasteBurst;
use crate::bottom_pane::textarea::TextArea;
use crate::bottom_pane::textarea::TextAreaState;

pub(super) struct DraftState {
    pub(super) textarea: TextArea,
    pub(super) textarea_state: RefCell<TextAreaState>,
    pub(super) is_bash_mode: bool,
    pub(super) pending_pastes: Vec<(String, String)>,
    pub(super) input_enabled: bool,
    pub(super) input_disabled_placeholder: Option<String>,
    pub(super) paste_burst: PasteBurst,
    pub(super) disable_paste_burst: bool,
    pub(super) mention_bindings: HashMap<u64, ComposerMentionBinding>,
    pub(super) recent_submission_mention_bindings: Vec<MentionBinding>,
    /// Set by a first Escape press on a non-empty composer (outside popups, bash mode, and Vim
    /// normal mode); a second, immediately-following Escape press while this is still set
    /// clears the draft. See the `KeyCode::Esc` branch in `handle_key_event_without_popup`.
    ///
    /// This exists as a two-step "arm, then confirm" gate rather than clearing on the first
    /// Escape so that a single leading Escape stays exactly as safe/inert as it already is
    /// everywhere else in this codebase (and its test suite) that presses Escape defensively
    /// before another action -- most notably `submit_current_composer` in
    /// chatwidget/tests/slash_commands.rs, which presses Escape once (to dismiss any popup)
    /// immediately followed by Enter to submit; a single-press "Escape clears" design would
    /// have silently turned that into "clear the draft, then submit nothing."
    pub(super) escape_clear_armed: bool,
}

impl DraftState {
    pub(super) fn new() -> Self {
        Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            is_bash_mode: false,
            pending_pastes: Vec::new(),
            input_enabled: true,
            input_disabled_placeholder: None,
            paste_burst: PasteBurst::default(),
            disable_paste_burst: false,
            mention_bindings: HashMap::new(),
            recent_submission_mention_bindings: Vec::new(),
            escape_clear_armed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ComposerMentionBinding {
    pub(super) sigil: char,
    pub(super) mention: String,
    pub(super) path: String,
}
