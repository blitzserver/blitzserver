

#[derive(Debug, Clone)]
pub struct MapImageFileReq {}

impl MapImageFileReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> MapImageFileReq {
        Self {}
    }
}

impl crate::packets::BodyContents for MapImageFileReq {
    const ID: u8 = 0xd;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
