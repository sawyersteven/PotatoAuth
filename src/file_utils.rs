use std::{fs, path::Path};

use crate::{Error, Result};

pub fn make_parent_dirs<P>(filepath: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let filepath = filepath.as_ref();
    let parent_path = match filepath.parent() {
        Some(p) => p,
        None => return Err(Error::new(format!("Cannot get parent directory of {:?}", filepath))),
    };

    return match fs::create_dir_all(parent_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::convert(e)),
    };
}

// Attempts to create directory tree and write contents to file
pub fn make_dirs_and_write<P, C>(filepath: P, contents: C) -> Result<()>
where
    P: AsRef<Path> + ToString,
    C: AsRef<[u8]>,
{
    let filepath = filepath.as_ref();
    match filepath.parent() {
        Some(dir) => match fs::create_dir_all(dir) {
            Ok(_) => match fs::write(filepath, contents) {
                Ok(_) => Ok(()),
                Err(e) => {
                    return Err(Error::convert(e));
                }
            },
            Err(e) => {
                return Err(Error::convert(e));
            }
        },
        None => {
            return Err(Error::new(format!("Cannot get parent directory of {:?}", filepath)));
        }
    }
}

pub fn file_exists<S>(filepath: &S) -> bool
where
    S: AsRef<std::ffi::OsStr> + ?Sized,
{
    return std::path::Path::new(filepath).exists();
}
