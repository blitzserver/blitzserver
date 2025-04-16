use std::{os::unix::prelude::OsStrExt};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone)]
pub struct DmFileDef {
    pub filename: String,
    pub folder: String,
    pub hash: u32,
}

#[derive(Debug, Clone)]
pub struct AstralPacketResp {
    pub dmfiles: Vec<DmFileDef>,
}

impl AstralPacketResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R, version: Option<u16>) -> AstralPacketResp {
        let name_bytes: Vec<u8> = vec![];
        let len = r.read_u16::<LittleEndian>().unwrap();
        let mut dmfiles = Vec::new();
        for i in 0..=len {
            let mut unk = [0u8; 4];
            r.read_exact(&mut unk).unwrap();
            let mut folder_name = String::new();
            let mut file_name = String::new();
            loop {
                let c = r.read_u8().unwrap();
                if c != 0 {
                    file_name.push(c as char);
                } else {
                    break;
                }
            }
            loop {
                let c = r.read_u8().unwrap();
                if c != 0 {
                    folder_name.push(c as char);
                } else {
                    break;
                }
            }
            let hash = r.read_u32::<LittleEndian>().unwrap();
            dmfiles.push(DmFileDef {
                folder: folder_name,
                filename: file_name,
                hash,
            });
        }
        AstralPacketResp { dmfiles }
    }
}

impl crate::packets::BodyContents for AstralPacketResp {
    const ID: u8 = 0x12;
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        if self.dmfiles.is_empty() {
            w.write_u32::<LittleEndian>(0xffffffff).unwrap();
        } else {
            w.write_u16::<LittleEndian>(self.dmfiles.len() as u16)
                .unwrap();
            for def in &self.dmfiles {
                w.write_all(&[0x6, 0x0, 0x17, 0x0]).unwrap();
                w.write_all(def.filename.as_bytes()).unwrap();
                w.write_u8(0).unwrap();
                if let Some(cversion) = version
                    && cversion > 561
                {
                    w.write_all(def.folder.as_bytes()).unwrap();
                    w.write_u8(0).unwrap();
                }
                w.write_u32::<LittleEndian>(def.hash).unwrap();
            }
        }
    }
}
