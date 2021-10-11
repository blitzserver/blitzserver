use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Clone)]
pub struct DmFileResp {
    pub contents: Vec<u8>,
}

impl std::fmt::Debug for DmFileResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DmFileResp")
            .field(
                "contents[0..32]",
                &self
                    .contents
                    .iter()
                    .take(32)
                    .map(|d| *d)
                    .collect::<Vec<u8>>(),
            )
            .finish()
    }
}

impl DmFileResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> DmFileResp {
        let _len = r.read_u32::<LittleEndian>().unwrap();
        let mut contents = vec![];
        r.read_to_end(&mut contents).unwrap();
        DmFileResp { contents }
    }
}

impl crate::packets::BodyContents for DmFileResp {
    const ID: u8 = 0x10;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u32::<LittleEndian>(self.contents.len() as u32).unwrap();
        w.write_all(&self.contents).unwrap();
    }
}
