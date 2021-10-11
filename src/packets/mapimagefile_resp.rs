use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Clone)]
pub struct MapImageFileResp {
    pub image_contents: Vec<u8>,
}
impl std::fmt::Debug for MapImageFileResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapImageFileResp")
            .field("image_contents[0..32]", &self.image_contents.iter().take(32).map(|d| *d).collect::<Vec<u8>>().iter())
            .finish()
    }
}

impl MapImageFileResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> MapImageFileResp {
        let mut image_contents = vec![];
        r.read_u16::<LittleEndian>().unwrap();
        r.read_u32::<LittleEndian>().unwrap();
        r.read_to_end(&mut image_contents).unwrap();
        MapImageFileResp { image_contents }
    }
}

impl crate::packets::BodyContents for MapImageFileResp {
    const ID: u8 = 0xe;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u32::<LittleEndian>(self.image_contents.len() as u32)
            .unwrap();
        w.write_all(&self.image_contents).unwrap();
    }
}
