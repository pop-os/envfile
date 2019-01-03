//! Libary for parsing environment files into an in-memory map.
//!
//! ```rust
//! extern crate envfile;
//!
//! use envfile::EnvFile;
//! use std::io;
//! use std::path::Path;
//!
//! fn main() -> io::Result<()> {
//!     let mut envfile = EnvFile::new(&Path::new("examples/test.env"))?;
//!
//!     for (key, value) in &envfile.store {
//!         println!("{}: {}", key, value);
//!     }
//!
//!     envfile.update("ID", "example");
//!     println!("ID: {}", envfile.get("ID").unwrap_or(""));
//!
//!     // envfile.write()?;
//!
//!     Ok(())
//! }
//! ```

extern crate snailquote;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str;

use snailquote::{unescape, escape};


/// An opened environment file, whose contents are buffered into memory.
pub struct EnvFile {
    /// Where the environment file exists in memory.
    pub path:  PathBuf,
    /// The data that was parsed from the file.
    pub store: BTreeMap<String, String>,
}

fn parse_line(entry: &[u8]) -> Option<(String, String)> {
    str::from_utf8(entry).ok().and_then(|l| {
        let line = l.trim();
        // Ignore comment line
        if line.starts_with('#') {
            return None;
        }
        let vline = line.as_bytes();
        vline.iter().position(|&x| x == b'=').and_then(|pos| {
            str::from_utf8(&vline[..pos]).ok().and_then(|x| {
                str::from_utf8(&vline[pos+1..]).ok().and_then(|right| {
                    // The right hand side value can be a quoted string
                    unescape(right).ok().map(|y| (x.to_owned(), y))
                })
            })
        })
    })
}

impl EnvFile {
    /// Open and parse an environment file.
    pub fn new<P: Into<PathBuf>>(path: P) -> io::Result<Self> {
        let path = path.into();
        let data = read(&path)?;
        let mut store = BTreeMap::new();

        let values = data.split(|&x| x == b'\n').flat_map(parse_line);

        for (key, value) in values {
            store.insert(key, value);
        }

        Ok(EnvFile { path, store })
    }

    /// Update or insert a key into the map.
    pub fn update(&mut self, key: &str, value: &str) {
        self.store.insert(key.into(), value.into());
    }

    /// Fetch a key from the map.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).as_ref().map(|x| x.as_str())
    }

    /// Write the map back to the original file.
    ///
    /// # Notes
    /// The keys are written in ascending order.
    pub fn write(&mut self) -> io::Result<()> {
        let mut buffer = Vec::with_capacity(1024);
        for (key, value) in &self.store {
            buffer.extend_from_slice(key.as_bytes());
            buffer.push(b'=');
            // The value may contain space and need to be quoted
            let v = escape(value.as_str()).into_owned();
            buffer.extend_from_slice(v.as_bytes());
            buffer.push(b'\n');
        }

        write(&self.path, &buffer)
    }
}

fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(&path).map_err(|why| io::Error::new(
        io::ErrorKind::Other,
        format!("unable to open file at {:?}: {}", path.as_ref(), why)
    ))
}

fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::create(&path).map_err(|why| io::Error::new(
        io::ErrorKind::Other,
        format!("unable to create file at {:?}: {}", path.as_ref(), why)
    ))
}

fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    open(path).and_then(|mut file| {
        let mut buffer = Vec::with_capacity(file.metadata().ok().map_or(0, |x| x.len()) as usize);
        file.read_to_end(&mut buffer).map(|_| buffer)
    })
}

fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    create(path).and_then(|mut file| file.write_all(contents.as_ref()))
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use super::*;
    use self::tempdir::TempDir;
    use std::collections::BTreeMap;
    use std::io::Write;

    const SAMPLE: &str = r#"DOUBLE_QUOTED_STRING="This is a 'double-quoted' string"
EFI_UUID=DFFD-D047
HOSTNAME=pop-testing
KBD_LAYOUT=us
KBD_MODEL=
KBD_VARIANT=
 LANG=en_US.UTF-8
OEM_MODE=0
# Intentional blank line

# Should ignore = operator in comment
RECOVERY_UUID=PARTUUID=asdfasd7asdf7sad-asdfa
ROOT_UUID=2ef950c2-5ce6-4ae0-9fb9-a8c7468fa82c
SINGLE_QUOTED_STRING='This is a single-quoted string'
"#;

    const SAMPLE_CLEANED: &str = r#"DOUBLE_QUOTED_STRING="This is a 'double-quoted' string"
EFI_UUID=DFFD-D047
HOSTNAME=pop-testing
KBD_LAYOUT=us
KBD_MODEL=
KBD_VARIANT=
LANG=en_US.UTF-8
OEM_MODE=0
RECOVERY_UUID=PARTUUID=asdfasd7asdf7sad-asdfa
ROOT_UUID=2ef950c2-5ce6-4ae0-9fb9-a8c7468fa82c
SINGLE_QUOTED_STRING='This is a single-quoted string'
"#;

    #[test]
    fn env_file_read() {
        let tempdir = TempDir::new("distinst_test").unwrap();
        let path = &tempdir.path().join("recovery.conf");

        {
            let mut file = create(path).unwrap();
            file.write_all(SAMPLE.as_bytes()).unwrap();
        }

        let env = EnvFile::new(path).unwrap();
        assert_eq!(&env.store, &{
            let mut map = BTreeMap::new();
            map.insert("HOSTNAME".into(), "pop-testing".into());
            map.insert("LANG".into(), "en_US.UTF-8".into());
            map.insert("KBD_LAYOUT".into(), "us".into());
            map.insert("KBD_MODEL".into(), "".into());
            map.insert("KBD_VARIANT".into(), "".into());
            map.insert("EFI_UUID".into(), "DFFD-D047".into());
            map.insert("RECOVERY_UUID".into(), "PARTUUID=asdfasd7asdf7sad-asdfa".into());
            map.insert("ROOT_UUID".into(), "2ef950c2-5ce6-4ae0-9fb9-a8c7468fa82c".into());
            map.insert("OEM_MODE".into(), "0".into());
            map.insert("DOUBLE_QUOTED_STRING".into(), "This is a 'double-quoted' string".into());
            map.insert("SINGLE_QUOTED_STRING".into(), "This is a single-quoted string".into());
            map
        });
    }

    #[test]
    fn env_file_write() {
        let tempdir = TempDir::new("distinst_test").unwrap();
        let path = &tempdir.path().join("recovery.conf");

        {
            let mut file = create(path).unwrap();
            file.write_all(SAMPLE.as_bytes()).unwrap();
        }

        let mut env = EnvFile::new(path).unwrap();
        env.write().unwrap();
        let copy: &[u8] = &read(path).unwrap();

        assert_eq!(copy, SAMPLE_CLEANED.as_bytes(), "Expected '{}' == '{}'", String::from_utf8_lossy(copy), SAMPLE_CLEANED);
    }
}
