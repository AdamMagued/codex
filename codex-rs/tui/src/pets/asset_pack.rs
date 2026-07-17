//! Built-in pet asset acquisition and cache ownership.
//!
//! Upstream, built-in pets are not checked into the TUI package as local
//! spritesheets: the TUI resolves them from a public OpenAI-hosted CDN
//! (`persistent.oaistatic.com`) on first use, verifies the downloaded file's
//! spritesheet geometry, and installs it into a versioned cache under
//! CODEX_HOME. kimcli must not fetch from OpenAI's infrastructure, and there
//! is no Kim-hosted equivalent CDN to point this at instead, so
//! `ensure_builtin_pet` below never performs that download: if a validated
//! spritesheet is not already present in the local cache, it fails with a
//! clear error instead of reaching `oaistatic.com`. Concretely this means:
//! built-in pets that were already cached before this change (or that ship a
//! pre-populated cache) keep working exactly as before; on a fresh
//! CODEX_HOME, selecting a built-in pet fails gracefully (the existing
//! `Result`-returning call sites already treat "asset unavailable" as a
//! non-fatal, user-visible error -- see `tui/src/chatwidget/pets.rs` and
//! `tui/src/app/pets.rs`, which report it through `AppEvent::PetSelectionLoaded`
//! / `AppEvent::ConfiguredPetLoaded` rather than panicking). Custom pets are
//! unaffected: their source of truth is already local, so this module never
//! touches them.
//!
//! This module deliberately stops at "a validated spritesheet exists at this
//! path". Higher layers remain responsible for deciding when previews should
//! block on a load and when a successfully loaded built-in pet is safe to
//! persist to config.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;

use super::catalog;

const PET_PACK_VERSION: &str = "v1";
const PET_PACK_DIR: &str = "cache/tui-pets";

pub(crate) fn builtin_spritesheet_path(codex_home: &Path, file: &str) -> PathBuf {
    pack_dir(codex_home).join("assets").join(file)
}

/// Ensure that a built-in pet's spritesheet is present and structurally valid.
///
/// kimcli does not fetch pet assets from a remote CDN (see the module doc
/// comment): this only ever validates an already-cached file and never
/// downloads one. Callers should treat any error here as "the asset is
/// unavailable", not as a partial install they can safely ignore.
pub(crate) fn ensure_builtin_pet(codex_home: &Path, pet: catalog::BuiltinPet) -> Result<()> {
    let destination = builtin_spritesheet_path(codex_home, pet.spritesheet_file);
    if validate_cached_spritesheet(&destination).is_ok() {
        return Ok(());
    }

    bail!(
        "built-in pet asset {} is not cached locally; kimcli does not download pet assets from a remote CDN",
        destination.display()
    );
}

fn pack_dir(codex_home: &Path) -> PathBuf {
    codex_home.join(PET_PACK_DIR).join(PET_PACK_VERSION)
}

fn validate_cached_spritesheet(path: &Path) -> Result<()> {
    let (width, height) =
        image::image_dimensions(path).with_context(|| format!("read {}", path.display()))?;
    if width != catalog::SPRITESHEET_WIDTH || height != catalog::SPRITESHEET_HEIGHT {
        bail!(
            "invalid pet spritesheet dimensions for {}: expected {}x{}, got {}x{}",
            path.display(),
            catalog::SPRITESHEET_WIDTH,
            catalog::SPRITESHEET_HEIGHT,
            width,
            height
        );
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn write_test_pack(codex_home: &Path) {
    let assets_dir = pack_dir(codex_home).join("assets");
    fs::create_dir_all(&assets_dir).unwrap();
    for pet in catalog::BUILTIN_PETS {
        let path = assets_dir.join(pet.spritesheet_file);
        catalog::write_test_spritesheet(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_builtin_pet_fails_without_a_remote_fetch_when_uncached() {
        // kimcli never downloads pet assets from a remote CDN (see the module doc
        // comment): on a fresh CODEX_HOME with no cached spritesheet, this must fail
        // gracefully rather than attempt any network call.
        let dir = tempfile::tempdir().unwrap();
        let pet = catalog::builtin_pet("dewey").unwrap();

        let err = ensure_builtin_pet(dir.path(), pet).expect_err("no cached asset, no CDN fetch");
        assert!(
            err.to_string().contains("does not download pet assets"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn ensure_builtin_pet_succeeds_when_already_cached() {
        let dir = tempfile::tempdir().unwrap();
        write_test_pack(dir.path());
        let pet = catalog::builtin_pet("dewey").unwrap();

        ensure_builtin_pet(dir.path(), pet).expect("cached asset should validate without a fetch");
    }

    #[test]
    fn write_test_pack_installs_all_builtins() {
        let dir = tempfile::tempdir().unwrap();

        write_test_pack(dir.path());

        for pet in catalog::BUILTIN_PETS {
            let path = builtin_spritesheet_path(dir.path(), pet.spritesheet_file);
            assert!(path.is_file());
            validate_cached_spritesheet(&path).unwrap();
        }
    }
}
