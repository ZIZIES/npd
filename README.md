# npd

A tiny command-line notepad. Named lists, plain text on disk, no dependencies.

```
$ npd create project-ideas
$ npd project-ideas add "something one" "something two" "etc"
$ npd project-ideas view
1 something one
2 something two
3 etc
$ npd project-ideas pop 2
$ npd project-ideas view
1 something one
2 etc
```

## Install

Needs a Rust toolchain (std only, no crates).

```
cargo build --release
install -Dm755 target/release/npd ~/.local/bin/npd
```

## Commands

| command | what it does |
| --- | --- |
| `npd create <name>` | make a new, empty notepad |
| `npd list` | list all notepads, sorted |
| `npd delete <name>` | delete a notepad (asks `[y/N]` first) |
| `npd <name> add <item>...` | append one or more items |
| `npd <name> view` | print items, numbered from 1 |
| `npd <name> pop <n>...` | remove item(s) by number; the index is required |
| `npd <name> insert <n> <item>` | put an item at position n, the rest shift down |
| `npd <name> edit <n> <item>` | replace item n |
| `npd <name> clear` | remove every item, keep the notepad |
| `npd <name> rename <new>` | rename the notepad |

Details:

- `pop 1 3` uses the numbering you see in `view`, not the numbering after the first removal. Duplicate numbers are removed once.
- `insert <n>` accepts 1 through length+1; length+1 appends.
- Anything invalid (bad number, unknown notepad, ...) prints an error to stderr, exits 1, and changes nothing.
- Notepad names use letters, digits, `_` and `-`, max 64 characters. `list`, `create` and `delete` are reserved because they are commands.
- Items can't contain newlines.

## Storage

One text file per notepad, one item per line:

```
$XDG_DATA_HOME/npd/<name>.txt     # default: ~/.local/share/npd/
```

So `cat`, `grep` and `git` all work on your notes. Saves write a temp file and `rename(2)` it over the original, so a crash can't leave a half-written notepad.

## Development

```
cargo test
```

The tests in `tests/cli.rs` run the real binary against a throwaway `XDG_DATA_HOME`.

## License

GPL-3.0, see [LICENSE](LICENSE).
