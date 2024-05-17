use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use num_traits::{FromPrimitive, ToPrimitive};

#[repr(i16)]
#[derive(Debug, Clone, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum RespCode {
    Success = 0x01,
    PasswordIncorrect = -0x01
}

#[derive(Debug, Clone)]
pub struct JoinGameResp {
    pub respcode: RespCode,
}

impl JoinGameResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> JoinGameResp {
        return Self {
            respcode: RespCode::from_i16(r.read_i16::<LittleEndian>().unwrap()).unwrap(),
        };
    }
}

impl crate::packets::BodyContents for JoinGameResp {
    const ID: u8 = 0x5;
    fn write<W: std::io::Write>(&self, w: &mut W, _version: Option<u16>) {
        w.write_i16::<LittleEndian>(self.respcode.to_i16().unwrap()).unwrap();
    }
}
