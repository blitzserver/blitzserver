use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Clone)]
pub struct MapWinterFileResp {
    pub winter_contents: Vec<u8>,
}
impl std::fmt::Debug for MapWinterFileResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapWinterFileResp")
            .field("winter_contents[0..32]", &self.winter_contents.iter().take(32).map(|d| *d).collect::<Vec<u8>>().iter())
            .finish()
    }
}

impl MapWinterFileResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> MapWinterFileResp {
        let mut winter_contents = vec![];
        r.read_u16::<LittleEndian>().unwrap();
        r.read_u32::<LittleEndian>().unwrap();
        r.read_to_end(&mut winter_contents).unwrap();
        MapWinterFileResp { winter_contents }
    }
}

impl crate::packets::BodyContents for MapWinterFileResp {
    const ID: u8 = 0x22;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u32::<LittleEndian>(self.winter_contents.len() as u32)
            .unwrap();
        w.write_all(&self.winter_contents).unwrap();
    }
}
