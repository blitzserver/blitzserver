use byteorder::ReadBytesExt;

#[derive(Debug, Clone)]
pub struct MapImageFileReq {
    pub filename: String,
}

impl MapImageFileReq {
    pub fn from_reader<R: std::io::Read>(r: &mut R, version: Option<u16>) -> MapImageFileReq {
        let mut filename = String::new();
        version.iter().for_each(|v| {
            let mut file_bytes = vec![];
            loop {
                let c = r.read_u8().unwrap();
                if c != 0 {
                    file_bytes.push(c);
                } else {
                    break;
                }
            }
            filename = String::from_utf8_lossy(&file_bytes).to_string();
            r.read_to_string(&mut filename).unwrap();
        });
        Self { filename }
    }
}

impl crate::packets::BodyContents for MapImageFileReq {
    const ID: u8 = 0xd;
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        match version {
            Some(v) if v >= 600 => {
                w.write_all(self.filename.as_bytes()).unwrap();
            }
            _ => {}
        }
    }
}
