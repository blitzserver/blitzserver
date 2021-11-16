use super::utils::ReadDom5Ext;
use super::DomSaveReadError;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub enum KingdomType {
    Human = 1,
    Computer = 2,
    Special = 3,
    Defeated = 4,
    DefeatedWithTurn = 5
}

#[derive(Debug)]
pub struct Kingdom {
    pub nation_id: u16,
    pub player_type: KingdomType,
    pub name: String,
}

impl Kingdom {
    pub fn read_contents<R: std::io::Read>(mut file: R) -> Result<Option<Self>, DomSaveReadError> {
        let nation_id = file.read_u16::<LittleEndian>()?;
        if nation_id == std::u16::MAX {
            return Ok(None);
        }
        file.read_u16::<LittleEndian>()?;
        let magic = file.read_u16::<LittleEndian>()?;
        if magic != 12546 {
            return Err(DomSaveReadError::BadSentinel((magic.into(), 12546)));
        }
        let mut unk = [0u8; 16];
        file.read_exact(&mut unk)?;
        let player_type = file.read_u16::<LittleEndian>()?;
        let mut unk1 = [0u8; 366];
        file.read_exact(&mut unk1)?;
        let name = file.read_domstring()?;
        let mut unk2 = [0u8; 1693];
        file.read_exact(&mut unk2)?;

        Ok(Some(Self {
            nation_id,
            player_type: match player_type {
                1 => KingdomType::Human,
                2 => KingdomType::Computer,
                3 => KingdomType::Special,
                std::u16::MAX => KingdomType::Defeated,
                65534 => KingdomType::DefeatedWithTurn,
                d => return Err(DomSaveReadError::BadKingdomType(d)),
            },
            name,
        }))
    }
}
