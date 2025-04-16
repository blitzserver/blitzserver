use byteorder::{LittleEndian, WriteBytesExt};
use num_traits::{FromPrimitive, ToPrimitive};
#[derive(Default, Debug, Clone, num_derive::FromPrimitive, num_derive::ToPrimitive)]
#[repr(i8)]
pub enum PlayerStatus {
    #[default]
    Open = 0,
    Closed = -1,
    Player = 1,
}
#[derive(smart_default::SmartDefault, Debug, Clone)]
pub struct GameInfoPlayer {
    pub status: PlayerStatus,
    #[default(0)]
    pub unk: u8,
    pub nation_id: u8,
    #[default(_code = "[0,0,0,0,0]")]
    pub unk1: [u8; 5],
    pub player_name: String,
    #[default(_code = "[0,0,0,0,0,0,0,0]")]
    pub unk2: [u8; 8],
}
#[derive(smart_default::SmartDefault, Debug, Clone)]
pub struct LobbyGameInfo {
    pub game_id: i32,
    #[default(1)]
    pub status: i16,
    #[default(_code = "[198, 134, 1, 0, 0, 0, 0, 0]")]
    pub unk: [u8; 8],
    pub game_name: String,
    //                     V thats the era                       
    #[default(_code = "[0, 1, 255, 255, 255, 255, 1, 0, 2, 0]")]
    pub unk1: [u8; 10],
    pub players: Vec<GameInfoPlayer>,
    #[default(_code="[64, 88, 0, 0]")]
    pub unk2: [u8; 4]
}

impl LobbyGameInfo {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyGameInfo {
        Self::default()
    }
}

impl crate::packets::BodyContents for LobbyGameInfo {
    const ID: u8 = 0x7;
    fn write<W: std::io::Write>(&self, w: &mut W, _version: Option<u16>) {
        w.write_i32::<LittleEndian>(self.game_id).unwrap();
        w.write_i16::<LittleEndian>(self.status).unwrap();
        w.write_all(&self.unk).unwrap();
        for c in self.game_name.chars() {
            w.write_u8(c as u8).unwrap();
        }
        w.write_u8(0).unwrap();
        w.write_all(&self.unk1).unwrap();
        for player in self.players.iter() {
            w.write_u8(player.status.to_u8().unwrap()).unwrap();
            w.write_u8(player.unk).unwrap();
            w.write_u8(player.nation_id).unwrap();
            w.write_all(&player.unk1).unwrap();
            for c in player.player_name.chars() {
                w.write_u8(c as u8).unwrap();
            }
            w.write_u8(0).unwrap();
            w.write_all(&player.unk2).unwrap();
        }
        w.write_all(&self.unk2).unwrap();
    }
}
