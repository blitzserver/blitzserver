use byteorder::{LittleEndian, ReadBytesExt};
#[derive(Debug, Clone)]
pub struct JoinGameReq {
    pub game_id: i32,
    pub pass: String,
}

impl JoinGameReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> JoinGameReq {
        return Self {
            game_id: r.read_i32::<LittleEndian>().unwrap(),
            pass: {
                let mut pass = String::new();
                while let Ok(c) = r.read_u8() {
                    if c == 0 {
                        break;
                    }
                    pass.push(c as char);
                }
                pass
            },
        };
    }
}

impl crate::packets::BodyContents for JoinGameReq {
    const ID: u8 = 0x4;
    fn write<W: std::io::Write>(&self, _w: &mut W, _version: Option<u16>) {}
}
