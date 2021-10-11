use byteorder::{LittleEndian, ReadBytesExt};
#[derive(Debug, Clone)]
pub struct SetTeamReq {
    pub nation_id: u16,
    pub team: u16,
}

impl SetTeamReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> SetTeamReq {
        SetTeamReq {
            nation_id: r.read_u16::<LittleEndian>().unwrap(),
            team: r.read_u16::<LittleEndian>().unwrap(),
        }
    }
}

impl crate::packets::BodyContents for SetTeamReq {
    const ID: u8 = 0x3a;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
