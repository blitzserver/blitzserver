use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct TwoHCrcResp {
    pub crcs: std::collections::HashMap<u16, u32>,
}

impl TwoHCrcResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> TwoHCrcResp {
        let mut crcs = std::collections::HashMap::new();
        loop {
            let nation_id = r.read_u16::<LittleEndian>().unwrap();
            if nation_id == 0xffff {
                r.read_u32::<LittleEndian>().unwrap();
                break;
            }
            crcs.insert(nation_id, r.read_u32::<LittleEndian>().unwrap());
        }
        TwoHCrcResp { crcs }
    }
}

impl crate::packets::BodyContents for TwoHCrcResp {
    const ID: u8 = 0x16;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        for (k, v) in self.crcs.iter() {
            w.write_u16::<LittleEndian>(*k).unwrap();
            w.write_u32::<LittleEndian>(*v).unwrap();
        }
        w.write_u16::<LittleEndian>(0xffff).unwrap();
        w.write_u32::<LittleEndian>(0x15b38).unwrap();
    }
}
