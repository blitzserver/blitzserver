
#[derive(Debug, Clone)]
pub struct PasswordsReq {
    remaining: Vec<u8>,
}

impl PasswordsReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> PasswordsReq {
        let mut remaining = vec![];
        r.read_to_end(&mut remaining).unwrap();
        PasswordsReq { remaining }
    }
}

impl crate::packets::BodyContents for PasswordsReq {
    const ID: u8 = 0x5;
    fn write<W: std::io::Write>(&self, _w: &mut W) {}
}
