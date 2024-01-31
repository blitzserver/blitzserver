use std::default;

use byteorder::ReadBytesExt;
#[derive(Debug, Clone)]
pub struct LobbyInfoResp {
    pub bytes: [u8; 32]
}

impl LobbyInfoResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyInfoResp {
        let mut resp = LobbyInfoResp { bytes: Default::default() };
        r.read_exact(&mut resp.bytes).unwrap();
        return resp;
    }
}

impl crate::packets::BodyContents for LobbyInfoResp {
    const ID: u8 = 0x1;
    fn write<W: std::io::Write>(&self, _w: &mut W, version: Option<u16>) {}
}
