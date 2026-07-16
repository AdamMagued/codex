// kimcli permanently disables the upstream-version check (see `tui/src/updates.rs` and
// `tui/src/update_action.rs`), so the helpers that used to compare against a fetched
// "latest" version (`is_newer`, `extract_version_from_latest_tag`) were removed as dead
// code. `is_source_build_version` is kept: it still gates the (now-inert) update-popup
// plumbing and is covered by tests below.
pub(crate) fn is_source_build_version(version: &str) -> bool {
    parse_version(version) == Some((0, 0, 0))
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut iter = v.trim().split('.');
    let maj = iter.next()?.parse::<u64>().ok()?;
    let min = iter.next()?.parse::<u64>().ok()?;
    let pat = iter.next()?.parse::<u64>().ok()?;
    Some((maj, min, pat))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn source_build_version_is_not_checked() {
        assert!(is_source_build_version("0.0.0"));
        assert!(!is_source_build_version("0.1.0"));
    }

    #[test]
    fn whitespace_is_ignored() {
        assert_eq!(parse_version(" 1.2.3 \n"), Some((1, 2, 3)));
    }
}
