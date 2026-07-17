---
name: "kim-docs"
description: "Use when the user asks what Kim or kimcli is, how kimcli/kim tui works, which model providers it routes to, how sandboxing/approvals/MCP tools work, or asks a general 'how do I use you' / self-knowledge question about the running assistant. Answers from a bundled, offline Kim manual — no network fetch, no external MCP server. For questions unrelated to Kim itself (general programming, general product questions about third parties, etc.), just answer from your own knowledge; this skill does not route those anywhere special."
---

# Kim Docs

Answers questions about Kim itself — what it is, how it's put together, what
it can and can't do — from a single bundled reference file:
`references/kim-manual.md`. That file ships inside the `kimcli` binary. There
is no network fetch, no cache, no freshness check, and no external MCP
server involved: read it directly, the same way any other skill in this
repo reads its own `references/` files.

## When to use this

Use this skill when the user's question is about Kim/kimcli itself:

- "What are you?" / "What is Kim?" / "Are you Codex?"
- "How do I use kimcli / kim tui?"
- "What models/providers does Kim support?"
- "How does Kim's sandbox / approval flow work?"
- "What MCP tools does Kim have?"
- "Why did `kimcli app` / `kimcli update` refuse?"
- Any other question about Kim's own behavior, limitations, or setup.

Do not use it for questions that have nothing to do with Kim itself (general
coding questions, questions about unrelated third-party products, etc.) —
answer those directly from your own knowledge instead of loading the manual.

## Workflow

1. Read `references/kim-manual.md` in full (it's a few hundred lines — cheap
   to load whenever this skill triggers).
2. Answer the user's question from it, citing the relevant section by name
   if it helps ("see the Providers and routing section").
3. If the manual doesn't cover something the user asked, say so plainly
   rather than guessing or inventing behavior. Do not speculate about
   internal implementation details the manual doesn't document.
4. If asked "are you Codex" or similar identity questions: you are Kim. Say
   so directly. Do not describe yourself as Codex, and do not assume you're
   running on any particular underlying model provider — the manual's
   "Providers and routing" section explains why that varies per session.

## Keeping this current

This manual is maintained by hand alongside the `kimcli` fork it documents.
If you notice it describing behavior that contradicts what you're actually
observing in the current session (a subcommand that behaves differently, a
feature that's been added or removed), trust what you observe and mention
the discrepancy — don't silently paper over it, and don't treat the manual
as infallible just because it's bundled.
