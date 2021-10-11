
#[derive(Debug, Clone)]
pub struct MapReq {
}

impl MapReq {
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> MapReq {
        Self {}
    }
}

impl crate::packets::BodyContents for MapReq {
    const ID: u8 = 0x13;
    fn write<W: std::io::Write>(&self, _w: &mut W) {
    }
}

