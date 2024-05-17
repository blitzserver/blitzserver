use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
#[derive(Default, Debug, Clone)]
pub struct ServerStatusResp {
    pub unkbytes: [u8; 5],
    pub unkshorts: [i16; 8],
    pub unkint: i32,
    pub unkshort: i16,
    pub unkbyte: u8,
    pub unkshorts2: [i16; 4],
    pub unkbyte1: u8,
    pub unkshorts3: [i16; 2],
}

impl ServerStatusResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> ServerStatusResp {
        return Self::default();
    }
}

impl crate::packets::BodyContents for ServerStatusResp {
    const ID: u8 = 0x13;
    fn write<W: std::io::Write>(&self, w: &mut W, _version: Option<u16>) {
        w.write_all(&self.unkbytes).unwrap();
        for i in self.unkshorts {
            w.write_i16::<LittleEndian>(i).unwrap();
        }
        w.write_i32::<LittleEndian>(self.unkint).unwrap();
        w.write_i16::<LittleEndian>(self.unkshort).unwrap();
        w.write_u8(self.unkbyte).unwrap();
        for i in self.unkshorts2 {
            w.write_i16::<LittleEndian>(i).unwrap();
        }
        w.write_u8(self.unkbyte1).unwrap();
        for i in self.unkshorts3 {
            w.write_i16::<LittleEndian>(i).unwrap();
        }
    }
}
