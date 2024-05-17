use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use std::{convert::TryInto, default::Default};

#[repr(i16)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum Status {
    Unk = 0,
    Preparing = 1,
    Running = 2,
    Finished = 4,
    Disabled5 = -5,
    Disabled3 = -3,
}
#[repr(i16)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum GameInfo {
    Normal = 0,
    Pass = 1,
    Mods = 2,
    PassAndMods = 3,
}
#[derive(Debug, Clone)]
pub struct WhatTheHellIsThis {
    pub unk: [u8; 9],
}
impl WhatTheHellIsThis {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> Self {
        Self {
            unk: {
                let mut b = [0u8; 9];
                r.read_exact(&mut b).unwrap();
                b
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct LobbyInfoGame {
    pub status: Status,
    pub has_pass: GameInfo,
    pub game_id: i32,
    pub unk: [u8; 12],
    pub name: String,
    pub motd: String,
}

impl LobbyInfoGame {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyInfoGame {
        Self {
            status: {
                let r = r.read_i16::<byteorder::LittleEndian>().unwrap();
                match Status::from_i16(r) {
                    Some(s) => s,
                    None => panic!("Unknown status {}", r),
                }
            },
            has_pass: {
                let r = r.read_i16::<byteorder::LittleEndian>().unwrap();
                match GameInfo::from_i16(r) {
                    Some(s) => s,
                    None => panic!("Unknown pass marker {}", r),
                }
            },
            game_id: r.read_i32::<LittleEndian>().unwrap(),
            unk: {
                let mut s: [u8; 12] = Default::default();
                r.read_exact(&mut s).unwrap();
                s
            },
            name: {
                let mut s: String = Default::default();
                while let Ok(c) = r.read_u8() {
                    if c == 0 {
                        break;
                    }
                    s.push(c as char);
                }
                s
            },
            motd: {
                let mut s: String = Default::default();
                while let Ok(c) = r.read_u8() {
                    if c == 0 {
                        break;
                    }
                    s.push(c as char);
                }
                s
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct LobbyInfoResp {
    pub unk: u8,
    pub motd: String,
    pub games: Vec<LobbyInfoGame>,
    // binary: Vec<u8>
}

impl LobbyInfoResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> LobbyInfoResp {
        return LobbyInfoResp {
            unk: r.read_u8().unwrap(),
            motd: {
                let mut s = String::new();
                while let Ok(c) = r.read_u8() {
                    if c == 0 {
                        break;
                    }
                    s.push(c as char);
                }
                s
            },
            games: {
                let game_count = r.read_i32::<LittleEndian>().unwrap();
                let mut games = vec![];
                for _ in 0..game_count {
                    let game = LobbyInfoGame::from_reader(r);
                    println!("Game {:?}", game);
                    games.push(game);
                }
                games
            },
        };
    }
}

impl crate::packets::BodyContents for LobbyInfoResp {
    const ID: u8 = 0x1;
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write_u8(self.unk).unwrap();
        for c in self.motd.chars() {
            w.write_u8(c as u8).unwrap();
        }
        w.write_u8(0).unwrap();
        w.write_i32::<LittleEndian>(self.games.len() as i32)
            .unwrap();
        for g in self.games.iter() {
            w.write_i16::<LittleEndian>(g.status.to_i16().unwrap())
                .unwrap();
            w.write_i16::<LittleEndian>(g.has_pass.to_i16().unwrap())
                .unwrap();
            w.write_i32::<LittleEndian>(g.game_id).unwrap();
            w.write_all(&g.unk).unwrap();
            for c in g.name.chars() {
                w.write_u8(c as u8).unwrap();
            }
            w.write_u8(0).unwrap();
            for c in g.motd.chars() {
                w.write_u8(c as u8).unwrap();
            }
            w.write_u8(0).unwrap();
        }
    }
}
