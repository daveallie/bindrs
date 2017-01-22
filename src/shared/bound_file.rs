use bincode::SizeLimit;
use bincode::rustc_serialize::{encode, decode};
use std::io::{Write, BufRead};

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
pub struct BoundFile {
    pub x: f32,
    pub y: f32,
}

impl BoundFile {
    pub fn to_writer<T: Write>(&self, writer: &mut T) {
        writer.write_all(&self.encode()[..]).unwrap();
        writer.write_all(&[0]).unwrap();
        writer.flush();
    }

    pub fn from_reader<T: BufRead>(reader: &mut T) -> BoundFile {
        let mut vec: Vec<u8> = vec![];
        reader.read_until(0, &mut vec).unwrap();
        BoundFile::decode(vec)
    }

    fn encode(&self) -> Vec<u8> {
        let encoded: Vec<u8> = encode(&self, SizeLimit::Infinite).unwrap();
        encoded
    }

    fn decode(bytes: Vec<u8>) -> BoundFile {
        let decoded: BoundFile = decode(&bytes[..]).unwrap();
        decoded
    }
}
