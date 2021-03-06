use bincode::{serialize, deserialize};
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use filetime::{self, FileTime};
use std::fs::{self, File};
use std::io::{Write, BufRead, Read};
use std::path::Path;

#[derive(Serialize, Deserialize, PartialEq)]
pub enum FileAction {
    CreateUpdate,
    Delete,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct BoundFile {
    pub action: FileAction,
    pub path: String,
    pub mtime: i64,
    pub contents: Vec<u8>,
}

impl BoundFile {
    pub fn to_writer<T: Write>(&self, writer: &mut T) {
        let encoded = &self.encode()[..];

        let len = encoded.len() as u64;
        let mut wtr = vec![];
        wtr.write_u64::<LittleEndian>(len).expect(
            "Couldn't write stream length to remote!",
        );

        writer.write_all(&wtr[..]).expect(
            "Couldn't write all bytes to remote!",
        );
        writer.write_all(encoded).expect(
            "Couldn't write all bytes to remote!",
        );
        writer.flush().expect("Couldn't flush all bytes to remote!");
    }

    pub fn from_reader<T: BufRead>(reader: &mut T) -> Self {
        let len: u64 = reader.read_u64::<LittleEndian>().expect(
            "Couldn't read stream length from remote!",
        );

        let mut vec: Vec<u8> = vec![];
        reader.take(len).read_to_end(&mut vec).expect(
            "Couldn't read all bytes from remote!",
        );
        Self::decode(&vec[..])
    }

    pub fn build_from_path_action(base_dir: &str, path: String, action: FileAction) -> Self {
        if action == FileAction::CreateUpdate {
            // Write or Create
            let mut vec: Vec<u8> = vec![];
            let mut file = File::open(format!("{}/{}", base_dir, path)).expect(
                "File does not exist locally, cannot build BoundFile",
            );
            file.read_to_end(&mut vec).expect(
                "Failed to read local file contents into BoundFile",
            );
            let mtime = FileTime::from_last_modification_time(
                &file.metadata().expect("Failed to read BoundFile metadata"),
            ).unix_seconds();
            Self {
                action,
                path,
                mtime,
                contents: vec,
            }
        } else {
            // Delete
            Self {
                action,
                path,
                mtime: 0,
                contents: vec![],
            }
        }
    }

    pub fn save_to_disk(&self, base_dir: &str) {
        let full_str_path = format!("{}/{}", base_dir, self.path);
        let full_path = Path::new(&full_str_path);
        let mut file_exists = full_path.exists();
        if file_exists && full_path.is_dir() {
            fs::remove_dir_all(&full_path).unwrap_or_else(|_| {
                panic!(
                    "Failed to remove folder where file should be: {}",
                    full_str_path
                )
            });
            file_exists = false;
        }

        if self.action == FileAction::CreateUpdate {
            // Write or Create
            let parent = full_path.parent().unwrap_or_else(|| {
                panic!("Failed to get parent for: {}", full_str_path)
            });
            fs::create_dir_all(&parent).unwrap_or_else(|_| {
                panic!("Failed to create parent directory for: {}", full_str_path)
            });
            let mut file = File::create(&full_path).unwrap_or_else(|_| {
                panic!("Failed to open/create file at: {}", full_str_path)
            });
            file.write_all(&self.contents[..]).unwrap_or_else(|_| {
                panic!("Failed to write all bytes to: {}", full_str_path)
            });
            file.sync_all().unwrap_or_else(|_| {
                panic!("Failed to sync contents at: {}", full_str_path)
            });

            let file_time = FileTime::from_unix_time(self.mtime, 0);
            filetime::set_file_times(full_path, file_time, file_time)
                .unwrap_or_else(|_| panic!("Failed to set file time at: {}", full_str_path));
        } else if file_exists {
            // Delete
            fs::remove_file(&full_path).unwrap_or_else(|_| panic!("Failed to delete file at: {}", full_str_path));
        }
    }

    fn encode(&self) -> Vec<u8> {
        serialize(&self).expect("Failed to encode BoundFile")
    }

    fn decode(bytes: &[u8]) -> Self {
        deserialize(bytes).expect("Failed to decode BoundFile")
    }
}
