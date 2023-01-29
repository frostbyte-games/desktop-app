use std::{env, fs};

#[derive(Debug)]
pub enum FileErrors {
    DoesNotExist,
}

pub fn get_path(relative_path: &str, create: bool) -> Result<String, FileErrors> {
    let path = format!("{}/{}", get_frostbyte_base_path().unwrap(), relative_path);
    if !std::path::Path::new(&path).exists() {
        match create {
            true => fs::create_dir(&path).unwrap(),
            false => return Err(FileErrors::DoesNotExist),
        }
    }

    Ok(path)
}

pub fn get_frostbyte_base_path() -> Result<String, String> {
    let path = format!("{}/.frostbyte", get_base_home_path().unwrap());
    if !std::path::Path::new(&path).exists() {
        fs::create_dir(&path).unwrap();
    }

    Ok(path)
}

pub fn get_base_home_path() -> Result<String, String> {
    match env::var("APPDATA") {
        Ok(val) => return Ok(val),
        Err(_) => match env::var("HOME") {
            Ok(val) => return Ok(val),
            Err(e) => {
                return Err(format!("Error: {}", e));
            }
        },
    }
}
