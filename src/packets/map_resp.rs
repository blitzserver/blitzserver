use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::BodyContents;
#[derive(Debug, Clone)]
pub struct MapResp {
    pub map_name: String,
    pub image_name: String,
    pub winter_name: Option<String>,
    pub map_len: u32,
    pub image_len: u32,
    pub winter_len: Option<u32>,
    pub map_crc: u32,
    pub image_crc: u32,
    pub winter_crc: Option<u32>,
}

impl MapResp {
    pub fn new(
        map_name: String,
        map_contents: Vec<u8>,
        image_name: String,
        image_contents: Vec<u8>,
        winter_name: Option<String>,
        winter_contents: Option<Vec<u8>>,
    ) -> Self {
        Self {
            map_name,
            map_len: map_contents.len() as u32,
            map_crc: crate::util::calculate_crc(&map_contents[..]),
            image_name,
            image_len: image_contents.len() as u32,
            image_crc: crate::util::calculate_crc(&image_contents[..]),
            winter_name: match winter_name {
                Some(winter_name) => Some(winter_name),
                _ => None,
            },
            winter_crc: match &winter_contents {
                Some(wc) => Some(crate::util::calculate_crc(&wc[..])),
                None => None,
            },
            winter_len: match winter_contents {
                Some(winter_contents) => Some(winter_contents.len() as u32),
                None => None,
            },
        }
    }
    pub fn from_reader<R: std::io::BufRead>(r: &mut R) -> MapResp {
        let mut mapname_bytes = vec![];
        r.read_until(0x0, &mut mapname_bytes).unwrap();
        let mapname = String::from_utf8_lossy(&mapname_bytes).to_string();
        let len = r.read_u32::<LittleEndian>().unwrap();
        let mut map_contents: Vec<u8> = vec![0; len as usize];
        r.read_exact(&mut map_contents).unwrap();
        let mut imagename_bytes = vec![];
        r.read_until(0x0, &mut imagename_bytes).unwrap();
        let imagename = String::from_utf8_lossy(&imagename_bytes).to_string();
        let len = r.read_u32::<LittleEndian>().unwrap();
        let mut image_contents: Vec<u8> = vec![0; len as usize];
        r.read_exact(&mut image_contents).unwrap();
        let mut wintername_bytes = vec![];
        match r.read_until(0x0, &mut wintername_bytes) {
            Ok(t) if t > 0 => {
                let wintername = String::from_utf8_lossy(&wintername_bytes).to_string();
                let len = r.read_u32::<LittleEndian>().unwrap();
                let mut winter_contents: Vec<u8> = vec![0; len as usize];
                r.read_exact(&mut winter_contents).unwrap();
                MapResp::new(
                    mapname,
                    map_contents,
                    imagename,
                    image_contents,
                    Some(wintername),
                    Some(winter_contents),
                )
            }
            _ => MapResp::new(mapname, map_contents, imagename, image_contents, None, None),
        }
    }
}

impl crate::packets::BodyContents for MapResp {
    const ID: u8 = 0x14;
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write_all(self.map_name.as_bytes()).unwrap();
        w.write_u8(0).unwrap();
        w.write_u32::<LittleEndian>(self.map_len).unwrap();
        w.write_u32::<LittleEndian>(self.map_crc).unwrap();

        w.write_all(self.image_name.as_bytes()).unwrap();
        w.write_u8(0).unwrap();
        w.write_u32::<LittleEndian>(self.image_len).unwrap();
        w.write_u32::<LittleEndian>(self.image_crc).unwrap();

        if let (Some(winter_name), Some(winter_len), Some(winter_crc)) =
            (&self.winter_name, &self.winter_len, &self.winter_crc)
        {
            w.write_all(winter_name.as_bytes()).unwrap();
            w.write_u8(0).unwrap();
            w.write_u32::<LittleEndian>(*winter_len).unwrap();
            w.write_u32::<LittleEndian>(*winter_crc).unwrap();
        } else {
            w.write_u8(0).unwrap();
            w.write_u32::<LittleEndian>(0).unwrap();
            w.write_u32::<LittleEndian>(0).unwrap();
        }
    }
}

#[derive(Clone, Debug)]
pub struct MapResp6 {
    planes: Vec<PlaneDef>,
}

impl crate::packets::BodyContents for MapResp6 {
    const ID: u8 = 0x14;

    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        for plane in &self.planes {
            plane.write(w, version);
        }
    }
}
#[derive(Clone, Debug)]
pub struct PlaneDef {
    plane_name: String,
    plane_file_name: String,
    unk: i32,
    unk1: i32,
    d6m_filename: String,
    unk2: i32,
    unk3: i32,
    plane_images: Vec<PlaneImage>,
}

impl PlaneDef {
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write_all(self.plane_name.as_bytes()).unwrap();
        w.write_all(self.plane_file_name.as_bytes()).unwrap();
        w.write_i32::<LittleEndian>(self.unk).unwrap();
        w.write_i32::<LittleEndian>(self.unk1).unwrap();
        w.write_all(self.d6m_filename.as_bytes()).unwrap();
        w.write_i32::<LittleEndian>(self.unk2).unwrap();
        w.write_i32::<LittleEndian>(self.unk3).unwrap();
        for image in &self.plane_images {
            image.write(w, version);
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlaneImage {
    name: String,
    unk: i32,
    unk1: i32,
    unk2: i8,
}

impl PlaneImage {
    pub fn from_reader<R: std::io::BufRead>(r: &mut R) -> Option<PlaneImage> {
        let mut name: String = "".to_owned();
        loop {
            let c = r.read_u8().unwrap();
            if c == 0 {
                break;
            }
            name.push(c as char);
        }
        if name == "" {
            return None;
        }
        let unk = r.read_i32::<LittleEndian>().unwrap();
        let unk1 = r.read_i32::<LittleEndian>().unwrap();
        let unk2 = r.read_i8().unwrap();
        return Some(PlaneImage {
            name,
            unk,
            unk1,
            unk2,
        });
    }
}

impl PlaneImage {
    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write_all(self.name.as_bytes()).unwrap();
        w.write_i32::<LittleEndian>(self.unk).unwrap();
        w.write_i32::<LittleEndian>(self.unk1).unwrap();
        w.write_i8(self.unk2).unwrap();
    }
}

impl PlaneDef {
    pub fn from_reader<R: std::io::BufRead>(r: &mut R) -> PlaneDef {
        let mut plane_name: String = "".to_owned();
        loop {
            let c = r.read_u8().unwrap();
            if c == 0 {
                break;
            }
            plane_name.push(c as char);
        }
        let mut plane_file_name: String = "".to_owned();
        loop {
            let c = r.read_u8().unwrap();
            if c == 0 {
                break;
            }
            plane_file_name.push(c as char);
        }
        let unk = r.read_i32::<LittleEndian>().unwrap();
        let unk1 = r.read_i32::<LittleEndian>().unwrap();
        let mut d6m_filename: String = "".to_owned();
        loop {
            let c = r.read_u8().unwrap();
            if c == 0 {
                break;
            }
            d6m_filename.push(c as char);
        }
        let unk2 = r.read_i32::<LittleEndian>().unwrap();
        let unk3 = r.read_i32::<LittleEndian>().unwrap();

        let mut plane_images = vec![];
        loop {
            match PlaneImage::from_reader(r) {
                None => break,
                Some(i) => plane_images.push(i),
            }
        }

        PlaneDef {
            plane_name,
            plane_file_name,
            unk,
            unk1,
            d6m_filename,
            unk2,
            unk3,
            plane_images,
        }
    }
}

impl MapResp6 {
    pub fn from_reader<R: std::io::BufRead>(r: &mut R) -> MapResp6 {
        let highestplane = r.read_i16::<LittleEndian>().unwrap();
        let mut planes: Vec<PlaneDef> = vec![];
        for _ in 0..=highestplane {
            planes.push(PlaneDef::from_reader(r));
        }

        MapResp6 { planes }
    }
}
