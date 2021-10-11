use byteorder::ReadBytesExt;
#[derive(Debug, Clone)]
pub struct NationsSelectedReq {
    pub nations_selected: Vec<i32>,
}

impl NationsSelectedReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> NationsSelectedReq {
        let mut nations_selected = vec![];
        let mut index = 0;
        loop {
            match r.read_u8() {
                Err(_) => break,
                Ok(b) => {
                    if b == 1 {
                        nations_selected.push(index);
                    }
                    index += 1;
                }
            }
        }
        NationsSelectedReq { nations_selected }
    }
}

impl crate::packets::BodyContents for NationsSelectedReq {
    const ID: u8 = 0x1;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
