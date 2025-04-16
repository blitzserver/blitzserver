use super::game_settings::GameSettings;
use super::{calendar::Calendar, kingdom::Kingdom, land::Land, DomSaveReadError};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct FthrContents {
    pub settings: GameSettings,
    calendars: Vec<Calendar>,
    sentinel: i32,
    unk2: i16,
    default_zoom: f32,
    lands: Vec<Land>,
    pub kingdoms: Vec<Kingdom>,
}

impl FthrContents {
    pub fn read_contents<R: std::io::Seek + std::io::Read>(
        mut f: R,
    ) -> Result<Self, DomSaveReadError> {
        Ok(FthrContents {
            settings: GameSettings::read_contents(&mut f)?,
            calendars: {
                let mut calendars = vec![];
                while let Some(calendar) = Calendar::read_contents(&mut f)? {
                    calendars.push(calendar);
                }
                calendars
            },
            sentinel: {
                let calendar_sentinel = f.read_i32::<LittleEndian>()?;
                if calendar_sentinel != 8283 {
                    return Err(DomSaveReadError::BadSentinel((
                        calendar_sentinel as u32,
                        8283,
                    )));
                }
                calendar_sentinel
            },
            unk2: f.read_i16::<LittleEndian>()?,
            default_zoom: f.read_f32::<LittleEndian>()?,
            lands: {
                let mut lands: Vec<Land> = vec![];
                while let Some(l) = Land::read_contents(&mut f)? {
                    lands.push(l);
                }
                lands
            },
            kingdoms: {
                let mut kingdoms: Vec<Kingdom> = vec![];
                while let Some(k) = Kingdom::read_contents(&mut f)? {
                    kingdoms.push(k);
                }
                kingdoms
            },
        })
    }
}
