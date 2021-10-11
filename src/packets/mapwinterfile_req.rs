


#[derive(Debug, Clone)]
pub struct MapWinterFileReq {}

impl MapWinterFileReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> MapWinterFileReq {
        Self {}
    }
}

impl crate::packets::BodyContents for MapWinterFileReq {
    const ID: u8 = 0x21;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
