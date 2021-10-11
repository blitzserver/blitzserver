use byteorder::{LittleEndian, ReadBytesExt};
#[derive(Debug, Clone)]
pub struct SetDiscReq {
    pub nation_id: u16,
    pub is_disc: u8,
}

impl SetDiscReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> SetDiscReq {
        SetDiscReq {
            nation_id: r.read_u16::<LittleEndian>().unwrap(),
            is_disc: r.read_u8().unwrap(),
        }
    }
}

impl crate::packets::BodyContents for SetDiscReq {
    const ID: u8 = 0x38;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
