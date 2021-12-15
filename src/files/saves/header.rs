use super::{DomSaveReadError, ReadDom5Ext};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Seek;

#[derive(Debug)]
pub struct Header {
    magic: [u8; 6],
    pub cdkey: u32,
    pub version: u32,
    pub turnnumber: i32,
    pub file_type: u32,
    unk: u32,
    pub gamename: String,
    pub nationid: i32,
    pub status: i32,
    unk1: u32,
    pub turnkey: u32,
    pub password: String,
    pub master_password: String,
    sentinel: u32,
}


impl Header {
    fn read_magic_marker<R: std::io::Read>(mut file: R) -> Result<[u8; 6], DomSaveReadError> {
        let mut magic_bytes = [0u8; 6];
        file.read_exact(&mut magic_bytes)?;
        if magic_bytes != [0x01, 0x02, 0x04, 0x44, 0x4f, 0x4d] {
            return Err(DomSaveReadError::BadMagic((
                magic_bytes,
                [0x01, 0x02, 0x04, 0x44, 0x4f, 0x4d],
            )));
        }
        return Ok(magic_bytes);
    }

    fn read_sentinel<R: std::io::Read>(mut file: R) -> Result<u32, DomSaveReadError> {
        let s = file.read_u32::<LittleEndian>()?;
        if s != 12546 {
            return Err(DomSaveReadError::BadSentinel((s, 12546)));
        }
        return Ok(s);
    }

    pub fn read_contents<R: Seek + std::io::Read>(mut file: R) -> Result<Header, DomSaveReadError> {
        return Ok(Header {
            magic: Self::read_magic_marker(&mut file)?,
            cdkey: file.read_u32::<LittleEndian>()?,
            version: file.read_u32::<LittleEndian>()?,
            turnnumber: file.read_i32::<LittleEndian>()?,
            file_type: file.read_u32::<LittleEndian>()?,
            unk: file.read_u32::<LittleEndian>()?,
            nationid: file.read_i32::<LittleEndian>()?,
            status: file.read_i32::<LittleEndian>()?,
            unk1: file.read_u32::<LittleEndian>()?,
            gamename: file.read_domstring()?,
            password: file.read_domsecret()?,
            master_password: file.read_domsecret()?,
            turnkey: file.read_u32::<LittleEndian>()?,
            sentinel: Self::read_sentinel(file)?
        });
    }
}
