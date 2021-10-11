use byteorder::{ReadBytesExt, WriteBytesExt};
#[derive(Debug, Clone)]
pub struct LoadingMessageResp {
    pub message: String,
}

impl LoadingMessageResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LoadingMessageResp {
        let mut message: String = String::new();
        while let Ok(c) = r.read_u8() {
            if c != 0 {
                message.push_str(&String::from_utf8_lossy(&[c]));
            }
        }
        LoadingMessageResp { message }
    }
}

impl crate::packets::BodyContents for LoadingMessageResp {
    const ID: u8 = 0x24; 
    fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_all(&self.message.as_bytes()).unwrap();
        w.write_u8(0).unwrap();
    }
}
