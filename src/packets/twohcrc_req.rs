
#[derive(Debug, Clone)]
pub struct TwoHCrcReq {
    remaining: Vec<u8>,
}

impl TwoHCrcReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> TwoHCrcReq {
        let mut remaining = vec![];
        r.read_to_end(&mut remaining).unwrap();
        TwoHCrcReq { remaining }
    }
}

impl crate::packets::BodyContents for TwoHCrcReq {
    const ID: u8 = 0x15;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
