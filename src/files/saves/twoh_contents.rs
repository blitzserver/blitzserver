use super::{ReadDom5Ext, DomSaveReadError};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct TwoHContents {
    pub pretender_name: String,
    pub pretender_unitid: u16 
}

impl TwoHContents {
    pub fn read_contents<R: std::io::Read>(mut file: R) -> Result<Self, DomSaveReadError> {
        let mut unk: [u8; 40] = [0; 40];
        file.read_exact(&mut unk)?;
        let pretender_unitid = file.read_u16::<LittleEndian>()?;
        let mut unk: [u8; 45] = [0; 45];
        file.read_exact(&mut unk)?;
        let pretender_name = file.read_domstring()?;
        Ok(Self {
            pretender_name,
            pretender_unitid,
        })
    }
}
