# devlogger

This is my approach to longer term memory: A "devlog". The idea is that writing a "DEVLOG" would likely come natural to an LLM because of its training data. This small CLI and skill (see [`./skill/`](./skill/)) provide rails to help it maintain a sectioned, chronological devlog (previous attempts with this as a mere instruction didn't work out because of difficulty maintaining order). 

The CLI part is appends to a markdown devlog with dates and numeric IDs to help the LLM operate with it.

There's no `delete` command intentionally. This can be done easily enough though by removing entries or section folders manually.


## Install

```sh
brew install bn-l/tap/devlogger
```

Or from source:

```sh
cargo install --path .
```

## Layout

Logs live under a `DEVLOG/` directory in whatever base directory you point
`devlogger` at (default: `cwd`, override with `-f <dir>`). Every entry
belongs to a section; each section has its own file:

```
<base>/DEVLOG/<section>/<section>-devlog.md
```

Sections are created the first time you write to them.

## Entry format

Each entry is a single markdown list item:

```
- <number> | <YYYY-MM-DD HH:MM:SS>: <text>
```

Lines that don't start with `- ` are left alone, so you can interleave
prose, headings, and blank lines in the markdown file — `devlogger` will
parse around them. `- ` lines, however, must match the canonical shape or
parsing fails loudly with a path and line number.

## Commands

```sh
devlogger new <section> <entry>            # append
devlogger list [<section>]                 # list with canonical numbers
devlogger sections                         # list all section names
devlogger update <section> <id> <entry>    # rewrite one entry's text
devlogger read <section> [<n>]             # dump file, or last <n> entries
```

A section name is required for `new`, `update`, and `read`. `list` takes
one optionally: without a section it prints every section's entries,
one per line, each prefixed with `[<section>] ` so you can tell at a
glance which section a line belongs to. Sections come out in
alphabetical order.

`list` prints each entry truncated to 80 terminal columns so rows fit
on one line (the `[<section>] ` prefix counts against that budget).
Wide glyphs (CJK, most emoji) count as two columns. When an entry is
longer than that, the row ends with ` (...N more)` where `N` is the
number of elided characters. Use `read` to dump entries verbatim.

`sections` prints one section name per line, sorted alphabetically.
Output is empty when there are no sections yet.

Multi-word entries must be quoted. `<id>` in `update` is either the
entry number, an exact `YYYY-MM-DD HH:MM:SS` timestamp, or a unique date
prefix (e.g. `2026-04-14`).

Section names must match `[a-z]+(-[a-z]+)*` — lowercase letters and
single hyphens, no digits, no underscores.

## Tests

```sh
cargo test
```
