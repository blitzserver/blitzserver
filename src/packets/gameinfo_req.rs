use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct GameInfoReq {
    version: u16,
    unk: u32,
}

impl GameInfoReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> GameInfoReq{
        let version = r.read_u16::<LittleEndian>().unwrap();
        let unk = r.read_u32::<LittleEndian>().unwrap();
        GameInfoReq { version, unk }
    }
}

impl crate::packets::BodyContents for GameInfoReq {
    const ID: u8 = 0x3d;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u16::<LittleEndian>(self.version).unwrap();
        w.write_u32::<LittleEndian>(0x4cfb0).unwrap();
    }
}
