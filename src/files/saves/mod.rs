pub mod header;
pub mod land;
pub mod kingdom;
pub mod trn_contents;
pub mod twoh_contents;
pub mod utils;

use std::io::{Read, Seek};
use thiserror::Error;
use utils::ReadDom5Ext;

pub use header::Header;
pub use trn_contents::TrnContents;
pub use twoh_contents::TwoHContents;

#[derive(Error, Debug)]
pub enum DomSaveReadError {
    #[error("IO Error")]
    ReadError(#[from] std::io::Error),
    #[error("Bad magic")]
    BadMagic((u64, u64)),
    #[error("Invalid filetype")]
    BadFileType(u32),
    #[error("Invalid kingdom type")]
    BadKingdomType(u16)
}

#[derive(Debug)]
pub enum SaveBody {
    TwoHContents(TwoHContents),
    TrnContents(TrnContents),
}

#[derive(Debug)]
pub struct SaveFile {
    pub header: Header,
    pub body: SaveBody,
}

impl SaveFile {
    pub fn read_contents<R: Seek + Read>(mut file: R) -> Result<Self, DomSaveReadError> {
        let header = Header::read_contents(&mut file)?;
        Ok(Self {
            body: match header.file_type {
                0 => SaveBody::TwoHContents(TwoHContents::read_contents(&mut file)?),
                1 => SaveBody::TrnContents(TrnContents::read_contents(&mut file)?),
                _ => return Err(DomSaveReadError::BadFileType(header.file_type)),
            },
            header,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn calc_crc() {
        let rec = crate::util::calculate_crc(std::io::Cursor::new(include_bytes!("../../../test_data/oeridiad5.map"))) as i32;
        println!("crc for map {}", rec); 
    }
}
