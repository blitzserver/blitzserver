use super::{DomSaveReadError, ReadDom5Ext};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Seek;

pub struct Header {
    pub gamename: String,
    pub turnnumber: i32,
    pub cdkey: u64,
    pub nationid: i32,
    pub turnkey: u32,
    pub password: String,
    pub master_password: String,
    pub status: i32,
    pub file_type: u32,
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwoH")
            .field("gamename", &self.gamename)
            .field("turnnumber", &self.turnnumber)
            .field("cdkey", &self.cdkey)
            .field("nationid", &self.nationid)
            .field("turnkey", &self.turnkey)
            .field("password", &self.password)
            .field("master_password", &self.master_password)
            .field("status", &self.status)
            .field("file_type", &self.file_type)
            .finish()
    }
}

impl Header {
    fn read_magic_marker<R: std::io::Read>(mut file: R) -> Result<(), DomSaveReadError> {
        let magic = file.read_u24::<LittleEndian>()?;
        if magic != 5066564 {
            return Err(DomSaveReadError::BadMagic((magic.into(), 5066564)));
        }
        return Ok(())
    }

    pub fn read_contents<R: Seek + std::io::Read>(mut file: R) -> Result<Header, DomSaveReadError> {
        let _unk = file.read_u24::<LittleEndian>()?;
        Self::read_magic_marker(&mut file)?;
        let cdkey = file.read_u64::<LittleEndian>()?;
        let turnnumber = file.read_i32::<LittleEndian>()?;
        let file_type = file.read_u32::<LittleEndian>()?;
        let _unk = file.read_u32::<LittleEndian>()?;
        let nationid = file.read_i32::<LittleEndian>()?;
        let status = file.read_i32::<LittleEndian>()?;
        file.read_i32::<LittleEndian>()?;
        let gamename = file.read_domstring()?;
        let password = file.read_domsecret()?;
        let masterpass = file.read_domsecret()?;
        let turnkey = file.read_u32::<LittleEndian>()?;
        Ok(Self {
            gamename,
            turnnumber,
            cdkey,
            nationid,
            turnkey,
            status,
            password,
            master_password: masterpass,
            file_type,
        })
    }
}
