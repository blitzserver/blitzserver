use byteorder::{LittleEndian, ReadBytesExt};

pub trait ReadDom5Ext: std::io::Read {
    fn read_domu8vec(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut contents = Vec::new();
        loop {
            let c = self.read_u8()?;
            if c == std::u8::MAX {
                break;
            }
            contents.push(c);
        }
        Ok(contents)
    }
    fn read_domu8vec_nt(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut contents = Vec::new();
        loop {
            let c = self.read_u8()?;
            if c == 0 {
                break;
            }
            contents.push(c);
        }
        Ok(contents)
    }
    fn read_domu16vec(&mut self) -> Result<Vec<u16>, std::io::Error> {
        let mut contents = Vec::new();
        loop {
            let c = self.read_u16::<LittleEndian>()?;
            if c == std::u16::MAX {
                break;
            }
            contents.push(c);
        }
        Ok(contents)
    }
    fn read_domu32vec(&mut self) -> Result<Vec<u32>, std::io::Error> {
        let mut contents = Vec::new();
        loop {
            let c = self.read_u32::<LittleEndian>()?;
            if c == std::u32::MAX {
                break;
            }
            contents.push(c);
        }
        Ok(contents)
    }
    fn read_domstring(&mut self) -> Result<String, std::io::Error> {
        let mut newstr = String::new();
        loop {
            let c = self.read_u8()?;
            if c ^ 0x4f == 0 {
                break;
            }
            newstr.push((c ^ 0x4f).into());
        }
        Ok(newstr)
    }

    fn read_domsecret(&mut self) -> Result<String, std::io::Error> {
        let mut newstr = String::new();
        let mut magic = 0x78;
        loop {
            let c: u8 = self.read_u8()? ^ magic;
            if c == 0 {
                break;
            }
            magic = (std::num::Wrapping(c) + std::num::Wrapping(magic)).0;
            newstr.push(c.into());
        }
        Ok(newstr)
    }
}

impl<R: std::io::Read + ?Sized> ReadDom5Ext for R {}
