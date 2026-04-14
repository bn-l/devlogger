# devlogger

An append-only markdown devlog CLI. Keeps a plain, human-readable log of
what you did, when, with stable numeric IDs — so you can grep it, diff it,
commit it, and edit it by hand.

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
`devlogger` at (default: `cwd`, override with `-f <dir>`):

```
<base>/DEVLOG/main-devlog.md
<base>/DEVLOG/<section>/<section>-devlog.md
```

The main log is implicit; sections are created the first time you write
to them.

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
devlogger new [<section>] <entry>           # append
devlogger list [<section>]                  # list with canonical numbers
devlogger sections                          # list all section names
devlogger update [<section>] <id> <entry>   # rewrite one entry's text
devlogger read [<section>] [<n>]            # dump file, or last <n> entries
```

`list` prints each entry truncated to 80 terminal columns so rows fit
on one line. Wide glyphs (CJK, most emoji) count as two columns. When
an entry is longer than that, the row ends with ` (...N more)` where
`N` is the number of elided characters. Use `read` to dump entries
verbatim.

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
