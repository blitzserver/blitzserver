
#[derive(Debug, Clone)]
pub struct PAReq {
    remaining: Vec<u8>,
}

impl PAReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> PAReq {
        let mut remaining = vec![];
        r.read_to_end(&mut remaining).unwrap();
        PAReq { remaining }
    }
}

impl crate::packets::BodyContents for PAReq {
    const ID: u8 = 0x19;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
