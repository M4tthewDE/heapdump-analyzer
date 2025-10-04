use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub enum Version {
    JavaProfile102,
}

impl Version {
    fn new(version_str: &str) -> Version {
        match version_str {
            "JAVA PROFILE 1.0.2" => Version::JavaProfile102,
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct Heap {
    pub version: Version,
}

pub fn parse(path: &Path) -> Heap {
    let mut file = File::open(path).unwrap();

    let mut version_buf = [0; 18];
    file.read_exact(&mut version_buf).unwrap();

    let version_str = String::from_utf8(version_buf.to_vec()).unwrap();

    Heap {
        version: Version::new(&version_str),
    }
}
