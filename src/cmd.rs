//! Subcommand dispatch and implementations.

use crate::store::{self, Notepad};
use std::io::{self, Write};

pub fn usage() -> String {
    "usage:\n  \
     npd create <name>\n  \
     npd list\n  \
     npd delete <name>\n  \
     npd <name> add <item>...\n  \
     npd <name> view\n  \
     npd <name> pop <n>...\n  \
     npd <name> insert <n> <item>\n  \
     npd <name> edit <n> <item>\n  \
     npd <name> clear\n  \
     npd <name> rename <new>"
        .to_string()
}

/// `args` is argv without the program name.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None => Err(usage()),
        Some("create") => create(&args[1..]),
        Some("list") => list(&args[1..]),
        Some("delete") => delete(&args[1..]),
        // anything else is a notepad name: `npd <name> <verb> ...`
        Some(name) => run_notepad(name, &args[1..]),
    }
}

fn run_notepad(name: &str, rest: &[String]) -> Result<(), String> {
    let Some((verb, args)) = rest.split_first() else {
        return Err(usage());
    };
    let mut pad = Notepad::load(name)?;
    match verb.as_str() {
        "add" => add(&mut pad, args),
        "view" => view(&pad, args),
        "pop" => pop(&mut pad, args),
        "insert" => insert(&mut pad, args),
        "edit" => edit(&mut pad, args),
        "clear" => clear(&mut pad, args),
        "rename" => rename(&mut pad, args),
        other => Err(format!("unknown command '{other}'\n{}", usage())),
    }
}

// ---- verbs that don't need a loaded notepad -------------------------------

fn create(rest: &[String]) -> Result<(), String> {
    match rest {
        [name] => Notepad::create(name).map(|_| ()),
        _ => Err("usage: npd create <name>".to_string()),
    }
}

fn list(rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err("usage: npd list".to_string());
    }
    for name in store::list_names()? {
        println!("{name}");
    }
    Ok(())
}

fn delete(rest: &[String]) -> Result<(), String> {
    let [name] = rest else {
        return Err("usage: npd delete <name>".to_string());
    };
    let pad = Notepad::load(name)?; // also errors if it doesn't exist
    eprint!("delete '{}' ({} items)? [y/N] ", pad.name, pad.items.len());
    io::stderr().flush().ok();
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|e| format!("can't read answer: {e}"))?;
    if matches!(answer.trim(), "y" | "Y" | "yes") {
        Notepad::delete(name)
    } else {
        eprintln!("aborted");
        Ok(())
    }
}

// ---- verbs on a loaded notepad --------------------------------------------

fn add(pad: &mut Notepad, items: &[String]) -> Result<(), String> {
    if items.is_empty() {
        return Err(format!("usage: npd {} add <item>...", pad.name));
    }
    // check everything first so a bad item doesn't leave a half-applied add
    items.iter().try_for_each(|i| check_item(i))?;
    pad.items.extend(items.iter().cloned());
    pad.save()
}

fn check_item(item: &str) -> Result<(), String> {
    if item.contains('\n') {
        return Err("items can't contain newlines".to_string());
    }
    Ok(())
}

fn view(pad: &Notepad, rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err(format!("usage: npd {} view", pad.name));
    }
    for (i, item) in pad.items.iter().enumerate() {
        println!("{} {item}", i + 1);
    }
    Ok(())
}

fn pop(pad: &mut Notepad, indexes: &[String]) -> Result<(), String> {
    if indexes.is_empty() {
        return Err(format!("usage: npd {} pop <n>...", pad.name));
    }
    // parse + range-check all of them before touching anything
    let mut idx = indexes
        .iter()
        .map(|s| parse_index(s, pad.items.len()))
        .collect::<Result<Vec<usize>, String>>()?;
    // remove high to low so earlier removals don't shift the later ones
    idx.sort_unstable_by(|a, b| b.cmp(a));
    idx.dedup();
    for i in idx {
        pad.items.remove(i);
    }
    pad.save()
}

/// `insert <n> <item>`: the new item ends up at position n, everything from
/// n onwards shifts down. n == len + 1 appends.
fn insert(pad: &mut Notepad, rest: &[String]) -> Result<(), String> {
    let [n, item] = rest else {
        return Err(format!("usage: npd {} insert <n> <item>", pad.name));
    };
    let at = parse_index(n, pad.items.len() + 1)?;
    check_item(item)?;
    pad.items.insert(at, item.clone());
    pad.save()
}

/// `edit <n> <item>`: replace item n.
fn edit(pad: &mut Notepad, rest: &[String]) -> Result<(), String> {
    let [n, item] = rest else {
        return Err(format!("usage: npd {} edit <n> <item>", pad.name));
    };
    let at = parse_index(n, pad.items.len())?;
    check_item(item)?;
    pad.items[at] = item.clone();
    pad.save()
}

fn clear(pad: &mut Notepad, rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err(format!("usage: npd {} clear", pad.name));
    }
    pad.items.clear();
    pad.save()
}

fn rename(pad: &mut Notepad, rest: &[String]) -> Result<(), String> {
    let [new] = rest else {
        return Err(format!("usage: npd {} rename <new>", pad.name));
    };
    pad.rename(new)
}

/// User-facing index (1-based) to Vec index (0-based), valid range is 1..=max.
fn parse_index(s: &str, max: usize) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid item number"))?;
    if max == 0 {
        return Err("the list is empty".to_string());
    }
    if n == 0 || n > max {
        return Err(format!("no item {n} (valid: 1-{max})"));
    }
    Ok(n - 1)
}
