

#[derive(Debug, Clone)]
pub struct DisconnectReq {}

impl DisconnectReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> DisconnectReq {
        DisconnectReq {}
    }
}

impl crate::packets::BodyContents for DisconnectReq {
    const ID: u8 = 0xb;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
