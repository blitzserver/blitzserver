
#[derive(Debug, Clone)]
pub struct NewGameReq {
//    unk: u16,
//    name: String,
//    master_password: String,
//    age: u8
}

impl NewGameReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> NewGameReq {
        Self {}
    }
}

impl crate::packets::BodyContents for NewGameReq {
    const ID: u8 = 0x2;
    fn write<W: std::io::Write>(&self, _w: &mut W, version: Option<u16>) {}
}
