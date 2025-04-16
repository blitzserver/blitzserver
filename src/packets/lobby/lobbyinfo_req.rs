
#[derive(Debug, Clone)]
pub struct LobbyInfoReq {
}

impl LobbyInfoReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyInfoReq {
        Self {}
    }
}

impl crate::packets::BodyContents for LobbyInfoReq {
    const ID: u8 = 0x0;
    fn write<W: std::io::Write>(&self, _w: &mut W, version: Option<u16>) {}
}
