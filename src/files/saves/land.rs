use super::utils::ReadDom5Ext;
use super::DomSaveReadError;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct Land {
    id: u32,
    name: String,
    name2: String,
    sites: [u16; 6],
    unrest: u8,
    owner: u16,
    owner_1: u16,
    owner_2: u16,
    defense: u8,
    unk_str1: String,
    unk_str2: String,
    term_list: Vec<u8>,
    event_log: String,
    recruiting_unitids: Vec<u16>,
    unk_vec: Vec<u16>
}
impl Land {
    pub fn read_contents<R: std::io::Read>(mut file: R) -> Result<Option<Self>, DomSaveReadError> {
        let id = file.read_u32::<LittleEndian>()?;
        if id == std::u32::MAX {
            return Ok(None)
        }
        let name = file.read_domstring()?;
        let name2 = file.read_domstring()?;
        file.read_u32::<LittleEndian>()?;
        let sites = {
            let mut dat = [0u16; 6];
            file.read_u16_into::<LittleEndian>(&mut dat)?;
            dat
        };
        let unrest = file.read_u8()?;
        let mut dat: [u8; 9] = [0u8; 9];
        file.read_exact(&mut dat)?;
        let owner = file.read_u16::<LittleEndian>()?;
        let owner_1 = file.read_u16::<LittleEndian>()?;
        let owner_2 = file.read_u16::<LittleEndian>()?;
        file.read_u32::<LittleEndian>()?;
        let defense = file.read_u8()?;
        let unk_str1 = {
            let mut dat: [u8; 109] = [0u8; 109];
            file.read_exact(&mut dat)?;
            file.read_domstring()?
        };
        let unk_str2 = file.read_domstring()?;
        let mut dat: [u8; 75] = [0u8; 75];
        file.read_exact(&mut dat)?;
        let term_list = file.read_domu8vec()?;
        file.read_u16::<LittleEndian>()?;
        let event_log = file.read_domstring()?;
        let mut unk = [0u8; 6];
        file.read_exact(&mut unk)?;
        let ruid_length = file.read_u16::<LittleEndian>()?;
        let mut recruiting_unitids = vec![0u16; ruid_length.into()];
        file.read_u16_into::<LittleEndian>(&mut recruiting_unitids)?;
        let mut unk = [0u8; 8];
        file.read_exact(&mut unk)?;
        let unk_vec = file.read_domu16vec()?;
        let mut unk = [0u8; 5];
        file.read_exact(&mut unk)?;
        Ok(Some(Self {
            id,
            name,
            name2,
            sites,
            unrest,
            owner,
            owner_1,
            owner_2,
            defense,
            unk_str1,
            unk_str2,
            term_list,
            event_log,
            recruiting_unitids,
            unk_vec
        }))
    }
}

// 229
