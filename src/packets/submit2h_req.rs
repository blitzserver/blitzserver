use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::files::saves::utils::ReadDom5Ext;
#[derive(Clone)]
pub struct Submit2hReq {
    pub nation_id: u8,
    pub unk1: u8,
    pub unk2: Vec<u8>,
    pub unk3: [u8; 5],
    pub twoh_contents: Vec<u8>,
}

impl std::fmt::Debug for Submit2hReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Submit2hReq")
            .field("nation_id", &self.nation_id)
            .field("unk1", &self.unk1)
            .field("unk2", &self.unk2)
            .field("unk3", &self.unk3)
            .field(
                "twoh_contents[0..32]",
                &self
                    .twoh_contents
                    .iter()
                    .take(32)
                    .map(|d| *d)
                    .collect::<Vec<u8>>(),
            )
            .finish()
    }
}

impl Submit2hReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> Submit2hReq {
        let nation_id = r.read_u8().unwrap();
        let unk1 = r.read_u8().unwrap();
        let unk2 = r.read_domu8vec_nt().unwrap();
        let mut unk3 = [0u8; 5];
        r.read_exact(&mut unk3).unwrap();
        let mut twoh_contents = vec![];
        r.read_to_end(&mut twoh_contents).unwrap();
        Submit2hReq {
            nation_id,
            unk1,
            unk2,
            unk3,
            twoh_contents,
        }
    }
}

impl crate::packets::BodyContents for Submit2hReq {
    const ID: u8 = 0x9;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u8(self.nation_id).unwrap();
        w.write_all(&[0x00, 0x00, 0x02c, 0xe6, 0x14, 0x00, 0x00])
            .unwrap();
        w.write_all(&self.twoh_contents).unwrap();
    }
}
