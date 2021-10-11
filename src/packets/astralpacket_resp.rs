use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone)]
pub struct AstralPacketResp {
    pub dmfiles: Vec<(String, u32)>,
}

impl AstralPacketResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> AstralPacketResp {
        let mut dmfiles = vec![];
        let mut name_bytes: Vec<u8> = vec![];
        loop {
            let mut unk: [u8; 6] = [0; 6];
            r.read_exact(&mut unk).unwrap();
            let c = r.read_u8().unwrap();
            if c != 0 {
                name_bytes.push(c);
            } else {
                break;
            }
        }
        let hash = r.read_u32::<LittleEndian>().unwrap();
        let name = String::from_utf8_lossy(&name_bytes);
        dmfiles.push((name.to_string(), hash));

        AstralPacketResp { dmfiles }
    }
}

impl crate::packets::BodyContents for AstralPacketResp {
    const ID: u8 = 0x12;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        if self.dmfiles.len() == 0 {
            w.write_u32::<LittleEndian>(0xffffffff).unwrap();
        } else {
            w.write_u16::<LittleEndian>(self.dmfiles.len() as u16).unwrap();
            for (name, hash) in &self.dmfiles {
                w.write_all(&[0x6, 0x0, 0x17, 0x0]).unwrap();
                w.write_all(name.as_bytes()).unwrap();
                w.write_u8(0).unwrap();
                w.write_u32::<LittleEndian>(*hash).unwrap();
            }
        }
    }
}
