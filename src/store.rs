//! On-disk storage: one text file per notepad, one item per line.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

/// Words that are subcommands, so they can't be notepad names
/// (`npd list view` would be ambiguous).
pub const RESERVED: [&str; 3] = ["list", "create", "delete"];

const MAX_NAME_LEN: usize = 64;

pub struct Notepad {
    pub name: String,
    pub items: Vec<String>,
}

/// Names are `[A-Za-z0-9_-]+`, which also rules out path traversal.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(format!("name must be 1-{MAX_NAME_LEN} characters"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!("invalid name '{name}': use only letters, digits, '_' and '-'"));
    }
    if RESERVED.contains(&name) {
        return Err(format!("'{name}' is a reserved word, pick another name"));
    }
    Ok(())
}

fn data_dir() -> Result<PathBuf, String> {
    if let Some(xdg) = env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("npd"));
    }
    let home = env::var_os("HOME").ok_or("neither XDG_DATA_HOME nor HOME is set")?;
    Ok(PathBuf::from(home).join(".local/share/npd"))
}

fn note_path(name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    Ok(data_dir()?.join(format!("{name}.txt")))
}

fn io_err(what: &str, e: std::io::Error) -> String {
    format!("{what}: {e}")
}

impl Notepad {
    /// Make a new, empty notepad. Fails if it already exists.
    pub fn create(name: &str) -> Result<Notepad, String> {
        let path = note_path(name)?;
        fs::create_dir_all(data_dir()?).map_err(|e| io_err("can't create data dir", e))?;
        // create_new = O_CREAT|O_EXCL: fails atomically if the file is already there
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Notepad { name: name.to_string(), items: Vec::new() }),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                Err(format!("notepad '{name}' already exists"))
            }
            Err(e) => Err(io_err("can't create notepad", e)),
        }
    }

    pub fn load(name: &str) -> Result<Notepad, String> {
        let path = note_path(name)?;
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err(format!("no notepad called '{name}'"));
            }
            Err(e) => return Err(io_err("can't read notepad", e)),
        };
        let items = text.lines().map(String::from).collect();
        Ok(Notepad { name: name.to_string(), items })
    }

    /// Write to `<name>.txt.tmp`, fsync, then rename over the real file.
    /// rename(2) is atomic, so a crash never leaves a half-written notepad.
    pub fn save(&self) -> Result<(), String> {
        let path = note_path(&self.name)?;
        let tmp = path.with_extension("txt.tmp");
        let write = || -> std::io::Result<()> {
            let mut f = File::create(&tmp)?;
            for item in &self.items {
                writeln!(f, "{item}")?;
            }
            f.sync_all()?;
            fs::rename(&tmp, &path)
        };
        write().map_err(|e| io_err("can't save notepad", e))
    }

    /// Rename the file on disk and update `self.name`. Fails if `new` is taken.
    pub fn rename(&mut self, new: &str) -> Result<(), String> {
        let from = note_path(&self.name)?;
        let to = note_path(new)?;
        if to.exists() {
            return Err(format!("notepad '{new}' already exists"));
        }
        fs::rename(&from, &to).map_err(|e| io_err("can't rename notepad", e))?;
        self.name = new.to_string();
        Ok(())
    }

    pub fn delete(name: &str) -> Result<(), String> {
        let path = note_path(name)?;
        fs::remove_file(&path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => format!("no notepad called '{name}'"),
            _ => io_err("can't delete notepad", e),
        })
    }
}

/// Sorted names of every notepad on disk.
pub fn list_names() -> Result<Vec<String>, String> {
    let entries = match fs::read_dir(data_dir()?) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("can't read data dir", e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| io_err("can't read data dir", e))?.path();
        if path.extension().is_some_and(|x| x == "txt") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}
