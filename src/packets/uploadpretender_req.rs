use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct UploadPretenderReq {
    pub nation_id: u16,
    pub unk: [u8; 5],
    pub pretender_contents: Vec<u8>,
}

impl UploadPretenderReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> UploadPretenderReq {
        let nation_id = r.read_u16::<LittleEndian>().unwrap();
        let mut unk = [0u8; 5];
        r.read_exact(&mut unk).unwrap();
        let mut pretender_contents = vec![];
        r.read_to_end(&mut pretender_contents).unwrap();
        UploadPretenderReq {
            nation_id,
            unk,
            pretender_contents,
        }
    }
}

impl crate::packets::BodyContents for UploadPretenderReq {
    const ID: u8 = 0x34;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u16::<LittleEndian>(self.nation_id).unwrap();
        w.write_all(&[0x2c, 0x6a, 0x1, 0x0, 0x0]).unwrap();
        w.write_all(&self.pretender_contents).unwrap();
    }
}
