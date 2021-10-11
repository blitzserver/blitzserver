use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone)]
pub struct ModFileReq {
    pub filename: String,
}

impl ModFileReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> ModFileReq {
        let mut file_bytes = vec![];
        loop {
            let c = r.read_u8().unwrap();
            if c != 0 {
                file_bytes.push(c);
            } else {
                break;
            }
        }
        ModFileReq {
            filename: String::from_utf8_lossy(&file_bytes).to_string(),
        }
    }
}

impl crate::packets::BodyContents for ModFileReq {
    const ID: u8 = 0x1b;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_all(self.filename.as_bytes()).unwrap();
        w.write_u8(0).unwrap();
    }
}
