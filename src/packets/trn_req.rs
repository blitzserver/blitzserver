use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct TrnReq {
    pub nation_desired: u8,
    remaining: Vec<u8>,
}

impl TrnReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> TrnReq {
        let mut remaining = vec![];
        let nation_desired = r.read_u8().unwrap();
        r.read_to_end(&mut remaining).unwrap();
        TrnReq { nation_desired, remaining }
    }
}

impl crate::packets::BodyContents for TrnReq {
    const ID: u8 = 0x7;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u8(self.nation_desired).unwrap();
        w.write_u16::<LittleEndian>(0).unwrap();
    }
}

