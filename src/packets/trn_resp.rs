use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Clone)]
pub struct TrnResp {
    pub trn_contents: Vec<u8>,
}
impl std::fmt::Debug for TrnResp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrnResp")
            .field("trn_contents[0..32]", &self.trn_contents.iter().take(32).map(|d| *d).collect::<Vec<u8>>().iter())
            .finish()
    }
}

impl TrnResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> TrnResp {
        let mut trn_contents = vec![];
        r.read_u16::<LittleEndian>().unwrap();
        r.read_u32::<LittleEndian>().unwrap();
        r.read_to_end(&mut trn_contents).unwrap();
        TrnResp { trn_contents }
    }
}

impl crate::packets::BodyContents for TrnResp {
    const ID: u8 = 0x8;
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write_all(&[0x01, 0x2b]).unwrap();
        w.write_u32::<LittleEndian>(self.trn_contents.len() as u32)
            .unwrap();
        w.write_all(&self.trn_contents).unwrap();
    }
}
