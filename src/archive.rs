use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Archive {
    Directory{
        name: String,
        children: Vec<Archive>
    },
    File{
        name: String,
        content: Vec<u8>,
    }
}

impl Archive {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Archive {
        bincode::deserialize(data).unwrap()
    }

    pub fn read_from_disk(path: &str) -> Archive {
        let full_path = fs::canonicalize(path).unwrap();
        let dir_name = full_path.file_name().unwrap().to_str().unwrap();
        if full_path.is_file() {
            return Self::File {
                name: dir_name.to_string(),
                content: fs::read(path).unwrap(),
            }
        }

        let children = fs::read_dir(path).unwrap()
            .map(|entry| entry.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
            .map(|child_path| Self::read_from_disk(&(path.to_string() + "/" + &child_path)))
            .collect::<Vec<_>>();

        Self::Directory {
            name: dir_name.to_string(),
            children,
        }
    }

    pub fn write_to_disk(&self, path: &str) {
        match self {
            Archive::File { name, content } if fs::metadata(format!("{}/{}", path, name)).is_err() => {
                fs::write(path.to_string() + "/" + name, content).unwrap_or(());
            },
            Archive::Directory { name, children } if fs::metadata(format!("{}/{}", path, name)).is_err() => {
                fs::create_dir(path.to_string() + "/" + name).unwrap_or(());
                children.iter().for_each(|child| child.write_to_disk(&(path.to_string() + "/" + name)));
            },
            Archive::File { name, .. } | Archive::Directory { name, .. } => println!("{} existiert bereits", name),
        }
    }

}