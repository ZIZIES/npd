//! End-to-end tests: run the real binary against a throwaway data dir.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Npd {
    data_dir: PathBuf,
}

struct Out {
    stdout: String,
    stderr: String,
    ok: bool,
}

impl Npd {
    /// Fresh, empty data dir per test.
    fn new(test: &str) -> Npd {
        let data_dir = std::env::temp_dir().join(format!("npd-test-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        Npd { data_dir }
    }

    fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Out {
        let mut child = Command::new(env!("CARGO_BIN_EXE_npd"))
            .args(args)
            .env("XDG_DATA_HOME", &self.data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
        let o = child.wait_with_output().unwrap();
        Out {
            stdout: String::from_utf8(o.stdout).unwrap(),
            stderr: String::from_utf8(o.stderr).unwrap(),
            ok: o.status.success(),
        }
    }

    fn run(&self, args: &[&str]) -> Out {
        self.run_with_stdin(args, "")
    }

    /// Run and assert success.
    fn ok(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(o.ok, "`npd {}` failed: {}", args.join(" "), o.stderr);
        o.stdout
    }

    /// Run, assert failure, return stderr.
    fn err(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(!o.ok, "`npd {}` should have failed", args.join(" "));
        o.stderr
    }
}

#[test]
fn spec_example() {
    let n = Npd::new("spec_example");
    n.ok(&["create", "project-ideas"]);
    n.ok(&["project-ideas", "add", "something one", "something two", "etc"]);
    assert_eq!(
        n.ok(&["project-ideas", "view"]),
        "1 something one\n2 something two\n3 etc\n"
    );
    n.ok(&["project-ideas", "pop", "2"]);
    assert_eq!(n.ok(&["project-ideas", "view"]), "1 something one\n2 etc\n");
}

#[test]
fn add_appends_across_calls() {
    let n = Npd::new("add_appends");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "one"]);
    n.ok(&["a", "add", "two", "three"]);
    assert_eq!(n.ok(&["a", "view"]), "1 one\n2 two\n3 three\n");
}

#[test]
fn view_empty_prints_nothing() {
    let n = Npd::new("view_empty");
    n.ok(&["create", "a"]);
    assert_eq!(n.ok(&["a", "view"]), "");
}

#[test]
fn add_rejects_newline_and_changes_nothing() {
    let n = Npd::new("add_newline");
    n.ok(&["create", "a"]);
    assert!(n.err(&["a", "add", "fine", "bad\nitem"]).contains("newline"));
    assert_eq!(n.ok(&["a", "view"]), "");
}

#[test]
fn add_without_items_errors() {
    let n = Npd::new("add_none");
    n.ok(&["create", "a"]);
    assert!(n.err(&["a", "add"]).contains("usage"));
}

#[test]
fn pop_without_index_errors() {
    let n = Npd::new("pop_none");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "x"]);
    assert!(n.err(&["a", "pop"]).contains("usage"));
    assert_eq!(n.ok(&["a", "view"]), "1 x\n");
}

#[test]
fn pop_bad_index_errors_and_changes_nothing() {
    let n = Npd::new("pop_bad");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "x", "y"]);
    for bad in ["0", "3", "-1", "abc", ""] {
        n.err(&["a", "pop", bad]);
    }
    // one bad index in a batch aborts the whole pop
    n.err(&["a", "pop", "1", "9"]);
    assert_eq!(n.ok(&["a", "view"]), "1 x\n2 y\n");
}

#[test]
fn pop_multiple_indexes_refer_to_original_numbering() {
    let n = Npd::new("pop_multi");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "a", "b", "c", "d", "e"]);
    n.ok(&["a", "pop", "1", "3"]);
    assert_eq!(n.ok(&["a", "view"]), "1 b\n2 d\n3 e\n");
}

#[test]
fn pop_duplicate_index_removes_once() {
    let n = Npd::new("pop_dup");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "a", "b", "c"]);
    n.ok(&["a", "pop", "2", "2"]);
    assert_eq!(n.ok(&["a", "view"]), "1 a\n2 c\n");
}

#[test]
fn unknown_notepad_errors() {
    let n = Npd::new("unknown");
    assert!(n.err(&["nope", "view"]).contains("no notepad called 'nope'"));
}

#[test]
fn reserved_names_and_duplicates_rejected() {
    let n = Npd::new("reserved");
    for r in ["list", "create", "delete"] {
        assert!(n.err(&["create", r]).contains("reserved"));
    }
    n.ok(&["create", "a"]);
    assert!(n.err(&["create", "a"]).contains("already exists"));
}

#[test]
fn list_is_sorted() {
    let n = Npd::new("list");
    assert_eq!(n.ok(&["list"]), "");
    n.ok(&["create", "b"]);
    n.ok(&["create", "a"]);
    assert_eq!(n.ok(&["list"]), "a\nb\n");
}

#[test]
fn delete_needs_confirmation() {
    let n = Npd::new("delete");
    n.ok(&["create", "a"]);
    assert!(n.run_with_stdin(&["delete", "a"], "n\n").ok);
    assert_eq!(n.ok(&["list"]), "a\n");
    assert!(n.run_with_stdin(&["delete", "a"], "y\n").ok);
    assert_eq!(n.ok(&["list"]), "");
}

#[test]
fn insert_shifts_items_down() {
    let n = Npd::new("insert");
    n.ok(&["create", "a"]);
    n.ok(&["a", "insert", "1", "first"]); // into an empty list
    n.ok(&["a", "add", "b", "c"]);
    n.ok(&["a", "insert", "2", "mid"]);
    n.ok(&["a", "insert", "5", "end"]); // len + 1 appends
    assert_eq!(n.ok(&["a", "view"]), "1 first\n2 mid\n3 b\n4 c\n5 end\n");
}

#[test]
fn insert_bad_args_error_and_change_nothing() {
    let n = Npd::new("insert_bad");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "x"]);
    n.err(&["a", "insert", "0", "y"]);
    n.err(&["a", "insert", "3", "y"]); // len + 2
    n.err(&["a", "insert", "abc", "y"]);
    n.err(&["a", "insert", "1"]); // missing item
    n.err(&["a", "insert", "1", "y", "z"]); // too many
    assert!(n.err(&["a", "insert", "1", "bad\nitem"]).contains("newline"));
    assert_eq!(n.ok(&["a", "view"]), "1 x\n");
}

#[test]
fn edit_replaces_in_place() {
    let n = Npd::new("edit");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "one", "tow", "three"]);
    n.ok(&["a", "edit", "2", "two"]);
    assert_eq!(n.ok(&["a", "view"]), "1 one\n2 two\n3 three\n");
}

#[test]
fn edit_bad_args_error_and_change_nothing() {
    let n = Npd::new("edit_bad");
    n.ok(&["create", "a"]);
    n.err(&["a", "edit", "1", "x"]); // empty list
    n.ok(&["a", "add", "x"]);
    n.err(&["a", "edit", "0", "y"]);
    n.err(&["a", "edit", "2", "y"]);
    n.err(&["a", "edit", "1"]);
    assert!(n.err(&["a", "edit", "1", "bad\nitem"]).contains("newline"));
    assert_eq!(n.ok(&["a", "view"]), "1 x\n");
}

#[test]
fn clear_empties_but_keeps_the_notepad() {
    let n = Npd::new("clear");
    n.ok(&["create", "a"]);
    n.ok(&["a", "add", "x", "y"]);
    n.ok(&["a", "clear"]);
    assert_eq!(n.ok(&["a", "view"]), "");
    assert_eq!(n.ok(&["list"]), "a\n");
    n.err(&["a", "clear", "extra"]);
}

#[test]
fn rename_moves_the_notepad() {
    let n = Npd::new("rename");
    n.ok(&["create", "a"]);
    n.ok(&["create", "taken"]);
    n.ok(&["a", "add", "x"]);
    n.ok(&["a", "rename", "b"]);
    assert_eq!(n.ok(&["list"]), "b\ntaken\n");
    assert_eq!(n.ok(&["b", "view"]), "1 x\n");
    n.err(&["a", "view"]);
}

#[test]
fn rename_rejects_bad_targets() {
    let n = Npd::new("rename_bad");
    n.ok(&["create", "a"]);
    n.ok(&["create", "taken"]);
    assert!(n.err(&["a", "rename", "taken"]).contains("already exists"));
    assert!(n.err(&["a", "rename", "list"]).contains("reserved"));
    n.err(&["a", "rename", "../evil"]);
    n.err(&["a", "rename"]);
    assert_eq!(n.ok(&["list"]), "a\ntaken\n");
}

#[test]
fn unknown_verb_errors() {
    let n = Npd::new("unknown_verb");
    n.ok(&["create", "a"]);
    assert!(n.err(&["a", "frobnicate"]).contains("unknown command"));
}
