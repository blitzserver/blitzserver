use byteorder::WriteBytesExt;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct PasswordsResp {
    nations_protected: HashMap<i32, bool>,
}

impl PasswordsResp {
    pub fn new(protected_nationids: &[i32]) -> Self {
        Self {
            nations_protected: protected_nationids.iter().fold(
                std::collections::HashMap::new(),
                |mut acc, cur| {
                    acc.insert(*cur, true);
                    acc
                },
            ),
        }
    }
    pub fn from_reader<R: std::io::Read>(_r: &mut R) -> PasswordsResp {
        PasswordsResp {
            nations_protected: HashMap::new(),
        }
    }
}

impl crate::packets::BodyContents for PasswordsResp {
    const ID: u8 = 0x6;
    fn write<W: std::io::Write>(&self, w: &mut W) {
        for i in 1..249 {
            match self.nations_protected.get(&i) {
                Some(_) => w.write_u8(1).unwrap(),
                None => w.write_u8(0).unwrap(),
            }
        }
    }
}
