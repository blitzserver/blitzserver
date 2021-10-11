
use byteorder::{WriteBytesExt};
#[derive(Debug, Clone)]
pub struct Submit2hResp {
}

impl Submit2hResp {
    pub fn new(_protected_nationids: &[i32]) -> Self {
        Self { }
    }
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> Submit2hResp {
        Submit2hResp { 
        }
    }
}

impl crate::packets::BodyContents for Submit2hResp {
    const ID: u8 = 0xa;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u8(1).unwrap();
    }
}

