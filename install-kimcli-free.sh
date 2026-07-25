#!/usr/bin/env bash
# One-shot installer for "kimcli-free" on Apple Silicon macOS.
# Clones the fork, builds kimcli, puts it on PATH, wires it to route through
# the openai-oauth proxy (free — uses your ChatGPT account), and installs a
# `kimcli-free` launcher that auto-starts the proxy.
set -euo pipefail

echo "==> kimcli-free installer (Apple Silicon macOS)"

# 1) Xcode command line tools (needed to compile)
if ! xcode-select -p >/dev/null 2>&1; then
  echo "==> Installing Xcode command line tools (a dialog will pop up)…"
  xcode-select --install || true
  echo "!! Finish the Xcode tools install, then re-run this same command."
  exit 1
fi

# 2) Rust toolchain
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Installing Rust…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env" 2>/dev/null || true

# 3) Node.js (for the openai-oauth proxy)
if ! command -v npx >/dev/null 2>&1; then
  echo "!! Node.js is required. Install it from https://nodejs.org and re-run this command."
  exit 1
fi

# 4) Clone / update the fork
SRC="$HOME/.kimcli-src"
if [ -d "$SRC/.git" ]; then
  echo "==> Updating source…"; git -C "$SRC" pull --ff-only
else
  echo "==> Cloning source…"; git clone --depth 1 https://github.com/AdamMagued/codex.git "$SRC"
fi

# 5) Build (LTO off keeps peak disk usage low; still an optimized release)
echo "==> Building kimcli (this takes ~20-30 min the first time)…"
cd "$SRC/codex-rs"
CARGO_PROFILE_RELEASE_LTO=off cargo build --release --bin kimcli

# 6) Put kimcli on PATH
echo "==> Installing kimcli to /usr/local/bin (may ask for your Mac password)…"
sudo ln -sf "$SRC/codex-rs/target/release/kimcli" /usr/local/bin/kimcli

# 7) Route kimcli through the openai-oauth proxy
mkdir -p "$HOME/.kim/codex"
cat > "$HOME/.kim/codex/config.toml" <<'CFG'
model = "gpt-5.6-sol"
model_provider = "openai-oauth"

[model_providers.openai-oauth]
name = "OpenAI (ChatGPT via openai-oauth)"
base_url = "http://127.0.0.1:10531/v1"
wire_api = "responses"
requires_openai_auth = false
CFG

# 8) `kimcli-free` launcher — starts the proxy (and ChatGPT login on first run)
sudo tee /usr/local/bin/kimcli-free >/dev/null <<'LAUNCH'
#!/bin/bash
set -e
PORT=10531
listening(){ nc -z 127.0.0.1 "$PORT" >/dev/null 2>&1; }
if ! listening; then
  if [ ! -f "$HOME/.codex/auth.json" ]; then
    echo "First-time setup: sign in with your ChatGPT account (a browser will open)…"
    npx -y openai-oauth@latest login
  fi
  echo "Starting the openai-oauth proxy…"
  npx -y openai-oauth@latest --detach >/dev/null 2>&1 &
  for _ in $(seq 1 40); do listening && break; sleep 1; done
fi
exec kimcli "$@"
LAUNCH
sudo chmod +x /usr/local/bin/kimcli-free

echo
echo "✅ Done!  From any project folder, run:   kimcli-free"
echo "   (first run signs you into ChatGPT and starts the proxy)"
