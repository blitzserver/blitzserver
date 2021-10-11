
#[derive(Debug, Clone)]
pub struct StartGameReq {}

impl StartGameReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> StartGameReq {
        StartGameReq {}
    }
}

impl crate::packets::BodyContents for StartGameReq {
    const ID: u8 = 0x36;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
