use byteorder::{ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct DmFileReq {
    pub filename: String,
}

impl DmFileReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> DmFileReq {
        let mut dmfile_bytes = vec![];
        loop {
            let c = r.read_u8().unwrap();
            if c != 0 {
                dmfile_bytes.push(c);
            } else {
                break;
            }
        }
        DmFileReq {
            filename: String::from_utf8_lossy(&dmfile_bytes).to_string(),
        }
    }
}

impl crate::packets::BodyContents for DmFileReq {
    const ID: u8 = 0xf;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_all(self.filename.as_bytes()).unwrap();
        w.write_u8(0).unwrap();
    }
}
