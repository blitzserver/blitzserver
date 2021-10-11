
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct TwoHReq {
    pub nation_desired: u8,
    remaining: Vec<u8>,
}

impl TwoHReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> TwoHReq {
        let mut remaining = vec![];
        let nation_desired = r.read_u8().unwrap();
        r.read_to_end(&mut remaining).unwrap();
        TwoHReq { nation_desired, remaining }
    }
}

impl crate::packets::BodyContents for TwoHReq {
    const ID: u8 = 0x17;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u8(self.nation_desired).unwrap();
        w.write_u16::<LittleEndian>(0).unwrap();
    }
}

