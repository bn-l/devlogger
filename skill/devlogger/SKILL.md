---
name: devlogger
description: Record a devlog entry after implementing a fix, feature, or non-trivial change. Uses the devlogger CLI to append an entry with a stable number and timestamp. Do NOT use before a change is implemented — entries are for work already done, not intentions.
---

# devlogger

Append-only markdown devlog, driven by the `devlogger` CLI. Every entry
belongs to a **section** — there is no implicit default log, so every
`new`, `update`, and `read` requires a section name.

## When to use

- You just finished a fix, feature, or non-trivial debug session.
- Before starting a task, to read prior context from the devlog
  (see "Reading prior context" below).

## When NOT to use

- You haven't implemented the change yet — do not pre-log intentions.
- A trivial one-line change that adds no future value to re-read.

## Reading prior context

At the start of a task, skim every section at once:

```sh
devlogger list 2>/dev/null | tail -n 50
```

`list` with no section prints one line per entry across **all**
sections, each prefixed with `[<section>] `, sorted by section. That's
usually enough context — dive into a specific section with
`devlogger read <section>` only if you need the full text.
`devlogger read <section>` with no `<n>` dumps the **entire** section
file verbatim; pass `<n>` to get only the last `<n>` entry lines.

If the command prints nothing (or fails with "devlog not found"), the
project has no devlog yet — that's fine, move on. Do NOT create an
empty section just to have one.

## Skimming and finding entries

`devlogger list` is the workhorse for both browsing and for looking up
an entry's number. Use it when you want to:

- **Skim the log** quickly without pulling every file into context.
- **Find the number** of an entry you want to edit, so you can pass it
  to `devlogger update <section> <id> "..."` without reading the full
  devlog first.

```sh
devlogger list            # every section, prefixed with [<section>]
devlogger list <section>  # just one section, no prefix
```

Rows truncated with ` (...N more)` mean the entry text was longer than
80 columns; run `devlogger read <section>` if you need it verbatim.
The `[<section>] ` prefix counts against the 80-column budget.

## Recording an entry

Add exactly one entry after the change is done. A section is required:

```sh
devlogger new <section> "<concise description of what it was + how you handled it>"
```

Quote the entry — it's a single argument. `devlogger` stamps the number
and date itself, so don't include them in the text.

### Choosing a section

First check which sections already exist:

```sh
devlogger sections
```

That prints one name per line, alphabetically (empty output means no
sections yet).

Rules for choosing:

- **Reuse an existing section** that matches the subsystem you worked
  on. This is the default behaviour — prefer reuse over creating.
- **Only create a new section** when no existing section is a
  reasonable fit. Pick a short, descriptive name for the subsystem
  (e.g. `parser`, `cli`, `store`). Avoid generic catch-all names like
  `misc` or `general` — they defeat the point of sections.
- If there are no sections at all yet, create the first one the same
  way: pick a name that describes the subsystem you touched.

Section names must match `[a-z]+(-[a-z]+)*`: lowercase letters and
single hyphens only. No digits, no underscores.

### What to include in the entry

Keep it terse. You are writing this for your future self. Useful signal:

- What the issue or task was (symptom or root cause).
- How you handled it (approach, not code).
- What didn't work first, if anything non-obvious.
- What resource/doc/file unblocked you, if it was obscure.

Skip: apologies, restating the task, broad project background.

### Example

```sh
devlogger new parser "parse_file silently dropped CRLF entries — detect_line_ending only looked at the first byte. Switched to scanning until the first \\n and checking the preceding byte. Covered by test_parse_crlf."
```

## Commands reference

```sh
devlogger new <section> <entry>             # append entry (section required)
devlogger list [<section>]                  # one-line-per-entry summary
devlogger sections                          # list all section names
devlogger update <section> <id> <entry>     # rewrite one entry's text
devlogger read <section>                    # dump the whole section verbatim
devlogger read <section> <n>                # or just the last <n> entry lines
devlogger -f <dir> <subcommand> ...         # target a different project dir
```

`list` is the only command where `<section>` is optional — omit it to
see every section at once with `[<section>] ` prefixes. `list` also
truncates each entry to 80 terminal columns and appends ` (...N more)`
when the text is elided; use `read` when you need entries verbatim.

`<id>` in `update` is the entry number from `list`, or a unique
`YYYY-MM-DD` (or fuller) date prefix.

## Never hand-edit the devlog

Go through the CLI for every change. Do not open files under `DEVLOG/`
and edit entry lines directly — use `devlogger update` to change an
entry's text, and `devlogger new` to add one. Hand-edits break
numbering invariants the tool relies on.

## Issues

If `devlogger` is not on PATH (`command not found`), install it:

```sh
brew install bn-l/tap/devlogger
```

Then re-run the original command.
