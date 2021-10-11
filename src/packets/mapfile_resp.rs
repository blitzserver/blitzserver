use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Clone)]
pub struct MapFileResp {
    pub map_contents: Vec<u8>,
}
impl std::fmt::Debug for MapFileResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapFileResp")
            .field(
                "map_contents[0..32]",
                &self
                    .map_contents
                    .iter()
                    .take(32)
                    .map(|d| *d)
                    .collect::<Vec<u8>>(),
            )
            .finish()
    }
}

impl MapFileResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> MapFileResp {
        let mut map_contents = vec![];
        r.read_u16::<LittleEndian>().unwrap();
        r.read_u32::<LittleEndian>().unwrap();
        r.read_to_end(&mut map_contents).unwrap();
        MapFileResp { map_contents }
    }
}

impl crate::packets::BodyContents for MapFileResp {
    const ID: u8 = 0x1e;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u32::<LittleEndian>(self.map_contents.len() as u32)
            .unwrap();
        w.write_all(&self.map_contents).unwrap();
    }
}
