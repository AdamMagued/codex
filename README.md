# kimcli — free AI coding in your terminal

**kimcli** is a coding agent for your terminal (a rebranded fork of OpenAI's Codex CLI)
that runs on your **ChatGPT account** instead of paid API credits. It routes every
request through a small local proxy ([openai-oauth](https://github.com/EvanZhouDev/openai-oauth))
that uses the ChatGPT sign-in you already have — no API key, no billing.

> It behaves exactly like Codex: you open a terminal in any project and chat with an
> agent that reads and edits your code, makes plans, and runs commands.

## Requirements

- A **Mac (Apple Silicon)** — these instructions target macOS. *(Linux works too; adjust paths.)*
- **[Node.js](https://nodejs.org)** installed (the proxy runs on it).
- A **ChatGPT account** — ideally a paid plan (Plus/Pro), since the strong coding
  models are gated by your plan.

## Install (one line)

Copy‑paste this into Terminal. It clones the repo, installs Rust if needed, builds
`kimcli`, puts it on your PATH, wires up the proxy routing, and installs a launcher:

```shell
curl -fsSL https://raw.githubusercontent.com/AdamMagued/codex/main/install-kimcli-free.sh -o /tmp/kimcli-install.sh && bash /tmp/kimcli-install.sh
```

The build takes ~20–30 minutes the first time. It will ask for your Mac password once.

## Use it

From **any** project folder:

```shell
kimcli-free
```

The first run opens your browser to sign into ChatGPT, starts the proxy in the
background, then drops you into the agent. After that, just run `kimcli-free` anywhere.

- Type what you want at the `›` prompt and press Enter.
- Use **`/model`** to switch model or raise reasoning effort (e.g. `gpt-5.6-sol` at `high`).

## Troubleshooting

**Every message says "We're currently experiencing high demand…"**
That almost always means the proxy has **no valid ChatGPT token** (they expire, or a
sign‑in was cancelled). Fix it by signing in again — and let the browser flow finish
(don't press Ctrl‑C):

```shell
npx -y openai-oauth@latest login
```

Confirm the token saved (this should print a file, not "No such file"):

```shell
ls -la ~/.codex/auth.json
```

Then start/refresh the proxy and run again:

```shell
npx -y openai-oauth@latest stop && npx -y openai-oauth@latest --detach
```

**"connection refused" / nothing responds** — the proxy isn't running. Start it:

```shell
npx -y openai-oauth@latest --detach
```

You only need to start the proxy again after a reboot (or after `openai-oauth stop`).
Running `kimcli-free` starts it for you automatically.

## How it works (in one picture)

```
kimcli  ──▶  openai-oauth proxy (127.0.0.1:10531)  ──▶  ChatGPT backend
 (agent)         (uses ~/.codex/auth.json)               (your account, free)
```

`~/.kim/codex/config.toml` (written by the installer) points kimcli at the proxy:

```toml
model = "gpt-5.6-sol"
model_provider = "openai-oauth"

[model_providers.openai-oauth]
name = "OpenAI (ChatGPT via openai-oauth)"
base_url = "http://127.0.0.1:10531/v1"
wire_api = "responses"
requires_openai_auth = false
```

> **Note:** Using a ChatGPT subscription for API‑style access is against OpenAI's
> terms of service and may be rate‑limited or blocked at their discretion. Use at
> your own risk.

---

<p align="center"><em>The original OpenAI Codex CLI documentation follows below.</em></p>

---

<p align="center"><strong>Codex CLI</strong> is a coding agent from OpenAI that runs locally on your computer.
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## Quickstart

### Installing and running Codex CLI

Run the following on Mac or Linux to install Codex CLI:

```shell
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

Run the following on Windows to install Codex CLI:

```shell
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
```

Codex CLI can also be installed via the following package managers:

```shell
# Install using npm
npm install -g @openai/codex
```

```shell
# Install using Homebrew
brew install --cask codex
```

Then simply run `codex` to get started.

<details>
<summary>You can also go to the <a href="https://github.com/openai/codex/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `codex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `codex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `codex-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `codex-x86_64-unknown-linux-musl`), so you likely want to rename it to `codex` after extracting it.

</details>

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
