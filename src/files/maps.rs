use crate::image::GenericImageView;
use std::io::BufRead;

pub struct MapFile {
    pub map_name: String,
    pub tga_filename: String,
    pub winter_filename: Option<String>,
    pub prov_count: i32,
    pub uwprov_count: i32,
    pub min_version: i32,
}

#[derive(Debug)]
enum MapFileReadErrorType {
    NoImageFile,
    NoName,
}
#[derive(Debug)]
struct MapFileReadError {
    err_type: MapFileReadErrorType,
}

impl std::fmt::Display for MapFileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "No valid {} in map file",
            match self.err_type {
                MapFileReadErrorType::NoImageFile => "#imagefile",
                MapFileReadErrorType::NoName => "#dom2title",
            }
        )
    }
}

impl std::error::Error for MapFileReadError {}

enum TerrainFlags {
    _SmallProvince = 1,
    _LargeProvince = 2,
    Sea = 4,
    _Freshwater = 8,
    _Highlands = 16,
    _Swamp = 32,
    _Waste = 64,
    _Forest = 128,
    _Farm = 256,
    _Nostart = 512,
    _ManySites = 1024,
    _DeepSea = 2048,
    _Cave = 4096,
    _Mountains = 4194304,
    _GoodThroneLocation = 16777216,
    _GoodStartLocation = 33554432,
    _BadThroneLocation = 67108864,
    _Warmer = 536870912,
    _Colder = 1073741824,
}

impl std::convert::TryFrom<&[u8]> for MapFile {
    type Error = Box<dyn std::error::Error>;
    fn try_from(contents: &[u8]) -> Result<Self, Self::Error> {
        let commented_regex = regex::Regex::new(r#"^ *--"#).unwrap();
        let nocomment_lines = contents
            .lines()
            .filter_map(Result::ok)
            .filter(|l| !commented_regex.is_match(l))
            .collect::<Vec<String>>()
            .join("\n");
        let tga_name = regex::Regex::new(r#"#imagefile "?([^"\n]+)"?"#)
            .ok()
            .and_then(|c| c.captures(&nocomment_lines))
            .and_then(|m| m.get(1))
            .and_then(|d| Some(d.as_str().to_owned()))
            .ok_or(MapFileReadError {
                err_type: MapFileReadErrorType::NoImageFile,
            })?;
        let name = regex::Regex::new(r#"#dom2title "?([^"\n]+)"?"#)
            .ok()
            .and_then(|c| c.captures(&nocomment_lines))
            .and_then(|m| m.get(1))
            .and_then(|d| Some(d.as_str().to_owned()))
            .ok_or(MapFileReadError {
                err_type: MapFileReadErrorType::NoName,
            })?;
        let winter_filename = regex::Regex::new(r#"#winterimagefile "?([^"\n]+)"?"#)
            .ok()
            .and_then(|c| c.captures(&nocomment_lines))
            .and_then(|m| m.get(1))
            .and_then(|d| Some(d.as_str().to_owned()));
        let min_version = regex::Regex::new(r#"#domversion ([0-9]+)"#)
            .ok()
            .and_then(|c| c.captures(&nocomment_lines))
            .and_then(|m| m.get(1))
            .and_then(|d| Some(d.as_str().to_owned()));

        let mut province_count: i32 = 0;
        let mut uw_count = 0;
        let capture_terrain = regex::Regex::new("#terrain ([0-9]+) ([0-9]+)").unwrap();
        for (_, terrain_type) in contents.lines().filter_map(Result::ok).filter_map(|l| {
            capture_terrain.captures(&l).and_then(|c| {
                Some((
                    c.get(1).unwrap().as_str().parse::<i32>().unwrap(),
                    c.get(2).unwrap().as_str().parse::<i32>().unwrap(),
                ))
            })
        }) {
            if terrain_type & TerrainFlags::Sea as i32 == TerrainFlags::Sea as i32 {
                uw_count = uw_count + 1;
            } else {
                province_count = province_count + 1;
            }
        }

        if province_count == 0 && uw_count == 0 {
            let mut set = std::collections::HashSet::new();
            let capture_neighbour = regex::Regex::new("#neighbour ([0-9]+) ([0-9]+)").unwrap();
            for prov_id in contents.lines().filter_map(Result::ok).filter_map(|l| {
                capture_neighbour
                    .captures(&l)
                    .and_then(|c| Some(c.get(1).unwrap().as_str().parse::<i32>().unwrap()))
            }) {
                set.insert(prov_id);
            }
            province_count = set.len() as i32;
        }

        Ok(MapFile {
            map_name: name,
            tga_filename: tga_name,
            winter_filename,
            min_version: min_version.as_deref().map(|c| str::parse::<i32>(c).ok()).flatten().unwrap_or(0),
            prov_count: province_count,
            uwprov_count: uw_count
        })
    }
}

pub struct ProvinceLocation {
    pub x: u32,
    pub y: u32,
    pub unit_x: f64,
    pub unit_y: f64
}
impl MapFile {
    pub fn load_provinces(img: &image::DynamicImage) -> Result<Vec<ProvinceLocation>, ()> {
        let mut provs = vec![];
        let width = img.width();
        let height = img.height();
        for (x, y, data) in img.pixels() {
            if data[0] == 255 && data[1] == 255 && data[2] == 255 {
                provs.push(ProvinceLocation {
                    x: x,
                    y: y,
                    unit_x: (x as f64) / (width as f64),
                    unit_y: (y as f64) / (height as f64)
                });
            }
        }
        provs.sort_by(|a, b| {
            if a.y < b.y {
                return std::cmp::Ordering::Greater;
            }
            if a.y > b.y {
                return std::cmp::Ordering::Less;
            }
            if a.x > b.x {
                return std::cmp::Ordering::Greater;
            }
            return std::cmp::Ordering::Less;
        });
        Ok(provs)
    }
}
