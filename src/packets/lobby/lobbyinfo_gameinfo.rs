#[derive(Default, Debug, Clone)]
pub struct LobbyInfoGameInfo {
    bytes: Vec<u8>,
}

impl LobbyInfoGameInfo {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyInfoGameInfo {
        Self {
            bytes: {
                let mut b = vec![];
                r.read_to_end(&mut b).unwrap();
                b
            },
        }
    }
}

impl crate::packets::BodyContents for LobbyInfoGameInfo {
    const ID: u8 = 0x23;
    fn write<W: std::io::Write>(&self, w: &mut W, _version: Option<u16>) {}
}
