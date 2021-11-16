use super::DomSaveReadError;
use super::game_settings::GameSettings;

#[derive(Debug)]
pub struct FthrContents {
    settings: GameSettings
}

impl FthrContents {
    pub fn read_contents<R: std::io::Seek + std::io::Read>(mut file: R) -> Result<Self, DomSaveReadError> {
        Ok(FthrContents {
            settings: GameSettings::read_contents(file)?,
        } )
    }
}
