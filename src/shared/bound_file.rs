use bincode::SizeLimit;
use bincode::rustc_serialize::{encode, decode};
use std::io::{Write, BufRead, Read};
use std::fs::{self, File};
use std::path::Path;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use filetime::{self, FileTime};

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
pub struct BoundFile {
    pub action: u8,
    pub path: String,
    pub mtime: u64,
    pub contents: Vec<u8>,
}

impl BoundFile {
    pub fn to_writer<T: Write>(&self, writer: &mut T) {
        let encoded = &self.encode()[..];

        let len = encoded.len() as u64;
        let mut wtr = vec![];
        wtr.write_u64::<LittleEndian>(len).unwrap();

        writer.write_all(&wtr[..]).unwrap();
        writer.write_all(encoded).unwrap();
        writer.flush().unwrap();
    }

    pub fn from_reader<T: BufRead>(reader: &mut T) -> BoundFile {
        let len: u64 = reader.read_u64::<LittleEndian>().unwrap();

        let mut vec: Vec<u8> = vec![];
        reader.take(len).read_to_end(&mut vec).unwrap();
        BoundFile::decode(&vec[..])
    }

    pub fn build_from_path_action(base_dir: &str, path: String, action: u8) -> BoundFile {
        if action == 0 {
            // Write or Create
            let mut vec: Vec<u8> = vec![];
            let mut file = File::open(format!("{}{}", base_dir, path)).unwrap();
            file.read_to_end(&mut vec).unwrap();
            let mtime = FileTime::from_last_modification_time(&file.metadata().unwrap())
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
        let full_str_path = format!("{}{}", base_dir, self.path);
        let full_path = Path::new(&full_str_path);
        let mut file_exists = full_path.exists();
        if file_exists && full_path.is_dir() {
            fs::remove_dir_all(&full_path).unwrap();
            file_exists = false;
        }

        if self.action == 0 {
            // Write or Create
            fs::create_dir_all(&full_path.parent().unwrap()).unwrap();
            let mut file = File::create(&full_path).unwrap();
            file.write_all(&self.contents[..]).unwrap();
            file.sync_all().unwrap();

            let file_time = FileTime::from_seconds_since_1970(self.mtime, 0);
            filetime::set_file_times(full_path, file_time, file_time).unwrap();

        } else {
            // Delete
            if file_exists {
                fs::remove_file(&full_path).unwrap();
            }
        }
    }

    fn encode(&self) -> Vec<u8> {
        let encoded: Vec<u8> = encode(&self, SizeLimit::Infinite).unwrap();
        encoded
    }

    fn decode(bytes: &[u8]) -> BoundFile {
        let decoded: BoundFile = decode(bytes).unwrap();
        decoded
    }
}
