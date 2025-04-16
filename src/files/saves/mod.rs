pub mod calendar;
pub mod fthr_contents;
pub mod game_settings;
pub mod header;
pub mod kingdom;
pub mod land;
pub mod trn_contents;
pub mod twoh_contents;
pub mod utils;

use std::io::{Read, Seek};
use thiserror::Error;
use utils::ReadDom5Ext;

pub use fthr_contents::FthrContents;
pub use header::Header;
pub use trn_contents::TrnContents;
pub use twoh_contents::TwoHContents;

#[derive(Error, Debug)]
pub enum DomSaveReadError {
    #[error("IO Error")]
    ReadError(#[from] std::io::Error),
    #[error("Bad magic")]
    BadMagic(([u8; 6], [u8; 6])),
    #[error("Bad sentinel")]
    BadSentinel((u32, u32)),
    #[error("Invalid filetype")]
    BadFileType(u32),
    #[error("Invalid kingdom type")]
    BadKingdomType(u16),
    #[error("Bad kingdom sentinel")]
    BadKingdomSentinel((u32, u32)),
}

#[derive(Debug)]
pub enum SaveBody {
    TwoHContents(TwoHContents),
    TrnContents(TrnContents),
    FthrContents(FthrContents),
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
            body: match (header.file_type, header.nationid) {
                (0, _) => SaveBody::TwoHContents(TwoHContents::read_contents(&mut file)?),
                (1, -1) => SaveBody::FthrContents(FthrContents::read_contents(&mut file)?),
                (1, _) => SaveBody::TrnContents(TrnContents::read_contents(&mut file)?),
                _ => return Err(DomSaveReadError::BadFileType(header.file_type)),
            },
            header,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pretender() {
        for path in std::fs::read_dir("./test_data/newlords")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game1() {
        for path in std::fs::read_dir("./test_data/game1")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game2() {
        for path in std::fs::read_dir("./test_data/game2")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game3() {
        for path in std::fs::read_dir("./test_data/game3")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game4() {
        for path in std::fs::read_dir("./test_data/game4")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game5() {
        for path in std::fs::read_dir("./test_data/game5")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game6() {
        for path in std::fs::read_dir("./test_data/game6")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game7() {
        for path in std::fs::read_dir("./test_data/game7")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }    
    #[test]
    fn test_game8() {
        for path in std::fs::read_dir("./test_data/game8")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
    #[test]
    fn test_game9() {
        for path in std::fs::read_dir("./test_data/game9")
            .unwrap()
            .flatten()
        {
            if path.file_type().unwrap().is_file() {
                println!("Deserializing {}", path.file_name().to_string_lossy());
                let file = std::fs::File::open(path.path()).unwrap();
                let deserialized = crate::files::saves::SaveFile::read_contents(file).unwrap();
                println!("Result: {:?}", deserialized);
            }
        }
    }
}
