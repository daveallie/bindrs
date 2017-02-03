use bincode::SizeLimit;
use bincode::rustc_serialize::{encode, decode};
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use filetime::{self, FileTime};
use std::fs::{self, File};
use std::io::{Write, BufRead, Read};
use std::path::Path;

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
pub enum FileAction {
    CreateUpdate,
    Delete,
}

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
pub struct BoundFile {
    pub action: FileAction,
    pub path: String,
    pub mtime: u64,
    pub contents: Vec<u8>,
}

impl BoundFile {
    pub fn to_writer<T: Write>(&self, writer: &mut T) {
        let encoded = &self.encode()[..];

        let len = encoded.len() as u64;
        let mut wtr = vec![];
        wtr.write_u64::<LittleEndian>(len).expect("Couldn't write stream length to remote!");

        writer.write_all(&wtr[..]).expect("Couldn't write all bytes to remote!");
        writer.write_all(encoded).expect("Couldn't write all bytes to remote!");
        writer.flush().expect("Couldn't flush all bytes to remote!");
    }

    pub fn from_reader<T: BufRead>(reader: &mut T) -> BoundFile {
        let len: u64 = reader.read_u64::<LittleEndian>()
            .expect("Couldn't read stream length from remote!");

        let mut vec: Vec<u8> = vec![];
        reader.take(len).read_to_end(&mut vec).expect("Couldn't read all bytes from remote!");
        BoundFile::decode(&vec[..])
    }

    pub fn build_from_path_action(base_dir: &str, path: String, action: FileAction) -> BoundFile {
        if action == FileAction::CreateUpdate {
            // Write or Create
            let mut vec: Vec<u8> = vec![];
            let mut file = File::open(format!("{}/{}", base_dir, path))
                .expect("File does not exist locally, cannot build BoundFile");
            file.read_to_end(&mut vec).expect("Failed to read local file contents into BoundFile");
            let mtime = FileTime::from_last_modification_time(&file.metadata()
                    .expect("Failed to read BoundFile metadata"))
                .seconds_relative_to_1970();
            BoundFile {
                action: action,
                path: path,
                mtime: mtime,
                contents: vec,
            }
        } else {
            // Delete
            BoundFile {
                action: action,
                path: path,
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
            fs::remove_dir_all(&full_path)
                .expect(&format!("Failed to remove folder where file should be: {}",
                                 full_str_path));
            file_exists = false;
        }

        if self.action == FileAction::CreateUpdate {
            // Write or Create
            let parent = full_path.parent()
                .expect(&format!("Failed to get parent for: {}", full_str_path));
            fs::create_dir_all(&parent)
                .expect(&format!("Failed to create parent directory for: {}", full_str_path));
            let mut file = File::create(&full_path)
                .expect(&format!("Failed to open/create file at: {}", full_str_path));
            file.write_all(&self.contents[..])
                .expect(&format!("Failed to write all bytes to: {}", full_str_path));
            file.sync_all().expect(&format!("Failed to sync contents at: {}", full_str_path));

            let file_time = FileTime::from_seconds_since_1970(self.mtime, 0);
            filetime::set_file_times(full_path, file_time, file_time)
                .expect(&format!("Failed to set file time at: {}", full_str_path));
        } else {
            // Delete
            if file_exists {
                fs::remove_file(&full_path)
                    .expect(&format!("Failed to delete file at: {}", full_str_path));
            }
        }
    }

    fn encode(&self) -> Vec<u8> {
        let encoded: Vec<u8> = encode(&self, SizeLimit::Infinite)
            .expect("Failed to encode BoundFile");
        encoded
    }

    fn decode(bytes: &[u8]) -> BoundFile {
        let decoded: BoundFile = decode(bytes).expect("Failed to decode BoundFile");
        decoded
    }
}
