use super::{DomSaveReadError, ReadDom5Ext};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek};

#[derive(Debug)]
pub struct GameSettingsMod {
    pub major: i16,
    pub minor: i16,
    pub name: String,
    pub unk1 : u64,
}

impl GameSettingsMod {
    pub fn read_contents<R: Seek + Read>(mut file: R) -> Result<Vec<Self>, DomSaveReadError> {
        let counter = file.read_i16::<LittleEndian>()?;
        let mut mods = vec![];
        let unk = file.read_u32::<LittleEndian>()?;
        for _ in 0..counter {
            mods.push(GameSettingsMod {
                major: file.read_i16::<LittleEndian>()?,
                minor: file.read_i16::<LittleEndian>()?,
                name: file.read_domstring()?,
                unk1: file.read_u64::<LittleEndian>()?,
            });
        }
        Ok(mods)
    }
}

#[derive(Debug)]
pub struct GameSettingsVictory {
    unk: i32,
    unk1: u8,
    unk2: i16,
    pub goldmul: u16,
    pub resmul: u16,
    pub supmul: u16,
}

impl GameSettingsVictory {
    pub fn read_contents<R: Seek + Read>(mut f: R) -> Result<Self, DomSaveReadError> {
        Ok(Self {
            unk: f.read_i32::<LittleEndian>()?,
            unk1: f.read_u8()?,
            unk2: f.read_i16::<LittleEndian>()?,
            goldmul: f.read_u16::<LittleEndian>()?,
            resmul: f.read_u16::<LittleEndian>()?,
            supmul: f.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct GameSettings {
    pub mode: u8,
    pub unk: u8,
    pub autoplay: u8,
    pub indepstr: u8,
    pub magicfreq: u8,
    pub eventrarity: u8,
    pub hofcount: u8,
    pub researchdiff: u8,
    pub mapwrap: u8,
    pub winnerid: u8,
    unk1: u8,
    counter: i16,
    pub victory_settings: Option<GameSettingsVictory>,
    pub unkshorts: Option<Vec<i16>>,
    pub era: i16,
    pub unk2: i32,
    pub unk3: i32,
    pub mods: Vec<GameSettingsMod>,
    pub unk4: u64,
}

impl GameSettings {
    pub fn read_contents<R: Seek + Read>(mut f: R) -> Result<Self, DomSaveReadError> {
        let tmp_counter;
        Ok(Self {
            mode: f.read_u8()?,
            unk: f.read_u8()?,
            autoplay: f.read_u8()?,
            indepstr: f.read_u8()?,
            magicfreq: f.read_u8()?,
            eventrarity: f.read_u8()?,
            hofcount: f.read_u8()?,
            researchdiff: f.read_u8()?,
            mapwrap: f.read_u8()?,
            winnerid: f.read_u8()?,
            unk1: f.read_u8()?,
            counter: {
                let counter = f.read_i16::<LittleEndian>()?;
                tmp_counter = counter * 2 + 8;
                counter
            },
            victory_settings: if tmp_counter > 5 {
                Some(GameSettingsVictory::read_contents(&mut f)?)
            } else {
                None
            },
            unkshorts: if tmp_counter == 0 {
                Some(
                    (0..=(tmp_counter - 6))
                        .map(|_| f.read_i16::<LittleEndian>())
                        .collect::<Result<Vec<i16>, _>>()?,
                )
            } else {
                None
            },
            era: f.read_i16::<LittleEndian>()?,
            unk2: f.read_i32::<LittleEndian>()?,
            unk3: f.read_i32::<LittleEndian>()?,
            mods: GameSettingsMod::read_contents(&mut f)?,
            unk4: f.read_u64::<LittleEndian>()?,
        })
    }
}
