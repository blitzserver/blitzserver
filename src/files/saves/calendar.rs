use super::{DomSaveReadError, ReadDom5Ext};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct Calendar {
    pub id: i32,
    pub unk1: i32,
    pub unk2: i32,
}

impl Calendar {
    pub fn read_contents<R: std::io::Seek + std::io::Read>(
        mut f: R,
    ) -> Result<Option<Self>, DomSaveReadError> {
        let id = f.read_i32::<LittleEndian>()?;
        if id == -1 {
            return Ok(None);
        }
        return Ok(Some(Self {
            id,
            unk1: f.read_i32::<LittleEndian>()?,
            unk2: f.read_i32::<LittleEndian>()?
        }));
    }
}
