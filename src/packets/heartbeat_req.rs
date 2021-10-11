
#[derive(Debug, Clone)]
pub struct HeartbeatReq {}

impl HeartbeatReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> HeartbeatReq {
        HeartbeatReq {}
    }
}

impl crate::packets::BodyContents for HeartbeatReq {
    const ID: u8 = 0x03;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
