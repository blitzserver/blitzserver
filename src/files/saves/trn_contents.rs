use super::utils::ReadDom5Ext;
use super::land::Land;
use super::kingdom::Kingdom;
use super::DomSaveReadError;

#[derive(Debug)]
pub struct TrnContents {
    pub map_tga: String,
    pub winter_tga: String,
    pub map_file: String,
    unk_string: String,
    unk_string1: String,
    unk_string2: String,
    unk_string3: String,
    unk_string4: String,
    pub lands: Vec<Land>,
    pub kingdoms: Vec<Kingdom>
}

impl TrnContents {
    pub fn read_contents<R: std::io::Read>(mut file: R) -> Result<Self, DomSaveReadError> {
        let mut unk = [0u8; 46];
        file.read_exact(&mut unk)?;
        let map_tga = file.read_domstring()?;
        let winter_tga = file.read_domstring()?;
        let map_file = file.read_domstring()?;
        let mut unk1 = [0u8; 48];
        file.read_exact(&mut unk1)?;
        let unk_string = file.read_domstring()?;
        let unk_string1 = file.read_domstring()?;
        let mut unk2 = [0u8; 146];
        file.read_exact(&mut unk2)?;
        let unk_string2 = file.read_domstring()?;
        let unk_string3 = file.read_domstring()?;
        let mut unk3 = [0u8; 78];
        file.read_exact(&mut unk3)?;
        let unk_string4 = file.read_domstring()?;
        let mut unk4 = [0u8; 23];
        file.read_exact(&mut unk4)?;
        let mut lands = Vec::new();
        while let Some(land) = Land::read_contents(&mut file)? {
            lands.push(land);
        }
        let mut kingdoms = Vec::new();
        while let Some(kingdom) = Kingdom::read_contents(&mut file)? {
            kingdoms.push(kingdom);
        }
        Ok(Self {
            map_tga,
            winter_tga,
            map_file,
            unk_string,
            unk_string1,
            unk_string2,
            unk_string3,
            unk_string4,
            lands,
            kingdoms,
        })
    }
}
