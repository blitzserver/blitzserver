
#[derive(Debug, Clone)]
pub struct DisconnectResp {}

impl DisconnectResp {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> DisconnectResp {
        DisconnectResp {}
    }
}

impl crate::packets::BodyContents for DisconnectResp {
    const ID: u8 = 0xc;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
