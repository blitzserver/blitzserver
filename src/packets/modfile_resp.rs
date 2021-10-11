use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, PartialEq)]
pub struct ModFileResp {
    pub contents: Vec<u8>,
    pub crc: u32,
    pub len: u32,
}
impl std::fmt::Debug for ModFileResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModFileResp")
            .field(
                "contents[0..32]",
                &self
                    .contents
                    .iter()
                    .take(32)
                    .map(|d| *d)
                    .collect::<Vec<u8>>()
                    .iter(),
            )
            .field("crc", &self.crc)
            .field("len", &self.len)
            .finish()
    }
}

impl ModFileResp {
    pub fn new(contents: Vec<u8>) -> Self {
        Self {
            len: contents.len() as u32,
            crc: crate::util::calculate_crc(&contents[..]),
            contents,
        }
    }
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> ModFileResp {
        let len = r.read_u32::<LittleEndian>().unwrap();
        let crc = r.read_u32::<LittleEndian>().unwrap();
        let mut contents = vec![];
        r.read_to_end(&mut contents).unwrap();
        assert_eq!(crate::util::calculate_crc(&contents[..]), crc);
        assert_eq!(len, contents.len() as u32);
        ModFileResp { contents, len, crc }
    }
}

impl crate::packets::BodyContents for ModFileResp {
    const ID: u8 = 0x1c;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u32::<LittleEndian>(self.contents.len() as u32)
            .unwrap();
        w.write_u32::<LittleEndian>(crate::util::calculate_crc(&self.contents[..]))
            .unwrap();
        w.write_all(&self.contents).unwrap();
    }
}
