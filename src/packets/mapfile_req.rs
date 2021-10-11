
#[derive(Debug, Clone)]
pub struct MapFileReq {}

impl MapFileReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> MapFileReq {
        Self {}
    }
}

impl crate::packets::BodyContents for MapFileReq {
    const ID: u8 = 0x1d;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
