use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone, Debug)]
pub struct GameInfoResp {
    pub unk1: u32,
    pub game_state: u8,
    pub game_name: String,
    pub era: i32,
    pub unk2: u32,
    pub disciples: bool,
    pub renaming: bool,
    pub unk3: u8,
    pub milliseconds_to_host: Option<u32>,
    pub unk4: u16,
    pub nation_statuses: std::collections::HashMap<i32, u8>,
    pub nation_discs: std::collections::HashMap<i32, u8>,
    pub nation_teams: std::collections::HashMap<i32, u8>,
    pub turn_statuses: std::collections::HashMap<i32, u8>,
    pub conn_statuses: std::collections::HashMap<i32, u8>,
    pub turn_number: u32,
    pub turnkey: u32,
    pub remaining: Vec<u8>,
}

const DISCIPLES_BIT_ID: u32 = 0b10000000;
const RENAMING_BIT_ID: u32 = 0b10000;

impl GameInfoResp {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> GameInfoResp {
        let unk1 = r.read_u32::<LittleEndian>().unwrap();
        let game_state = r.read_u8().unwrap();
        let mut buf = [0u8; 1];
        let mut name = String::new();
        loop {
            r.read_exact(&mut buf).unwrap();
            if buf[0] == 0 {
                break;
            }
            name.push(buf[0] as char);
        }
        let era = r.read_u8().unwrap();
        let unk2 = r.read_u32::<LittleEndian>().unwrap();
        let disciples = unk2 & DISCIPLES_BIT_ID == DISCIPLES_BIT_ID;
        let renaming = unk2 & RENAMING_BIT_ID == RENAMING_BIT_ID;
        let unk3 = r.read_u8().unwrap();
        let milliseconds_to_host = match r.read_u32::<LittleEndian>() {
            Ok(0xffffffff) => None,
            Ok(a) => Some(a),
            _ => None,
        };
        let unk4 = r.read_u16::<LittleEndian>().unwrap();
        let mut nation_statuses: std::collections::HashMap<i32, u8> =
            std::collections::HashMap::new();
        for i in 1..500 {
            match r.read_u8() {
                Ok(d) if d != 0 => {
                    nation_statuses.insert(i, d);
                }
                _ => {}
            }
        }
        let mut turn_statuses: std::collections::HashMap<i32, u8> =
            std::collections::HashMap::new();
        let mut conn_statuses: std::collections::HashMap<i32, u8> =
            std::collections::HashMap::new();
        let mut nation_discs: std::collections::HashMap<i32, u8> = std::collections::HashMap::new();
        let mut nation_teams: std::collections::HashMap<i32, u8> = std::collections::HashMap::new();

        if game_state == 1 {
            for i in 1..500 {
                match r.read_u8() {
                    Ok(team) if team > 0 => {
                        nation_teams.insert(i, team);
                    }
                    _ => {}
                }
            }
            for i in 1..500 {
                match r.read_u8() {
                    Ok(disc) if disc > 0 => {
                        nation_discs.insert(i, disc);
                    }
                    _ => {}
                }
            }
        } else {
            for i in 0..500 {
                match r.read_u8() {
                    Ok(status) if status != 0 => {
                        turn_statuses.insert(i, status);
                    }
                    _ => {}
                }
            }
            for i in 0..500 {
                match r.read_u8() {
                    Ok(1) => {
                        conn_statuses.insert(i, 1);
                    }
                    _ => {}
                }
            }
        }
        let turn_number = r.read_u32::<LittleEndian>().unwrap();
        r.read_u8().unwrap();
        let turnkey = r.read_u32::<LittleEndian>().unwrap();

        let mut remaining = vec![];
        r.read_to_end(&mut remaining).unwrap();
        GameInfoResp {
            unk1,
            game_state,
            game_name: name,
            era: era.into(),
            unk2,
            disciples,
            renaming,
            unk3,
            milliseconds_to_host,
            unk4,
            nation_statuses,
            turn_statuses,
            nation_discs,
            nation_teams,
            conn_statuses,
            turn_number,
            turnkey,
            remaining,
        }
    }
}
impl crate::packets::BodyContents for GameInfoResp {
    const ID: u8 = 0x4;

    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        w.write(&[0xc3, 0x1, 0, 0]).unwrap();
        w.write_u8(self.game_state).unwrap();
        w.write(self.game_name.as_bytes()).unwrap();
        w.write(&[0u8]).unwrap(); // Null terminator
        w.write(&[self.era as u8]).unwrap();
        let mut unk_bit_map = 0x840;
        if self.disciples {
            unk_bit_map = unk_bit_map | DISCIPLES_BIT_ID;
        }
        if self.renaming {
            unk_bit_map = unk_bit_map | RENAMING_BIT_ID;
        }
        w.write_u32::<LittleEndian>(unk_bit_map).unwrap();
        w.write_u8(0x2d).unwrap();
        match self.milliseconds_to_host {
            Some(t) => w.write_u32::<LittleEndian>(t).unwrap(),
            None => w.write_u32::<LittleEndian>(0xffffffff).unwrap(),
        };

        w.write_u8(0).unwrap();
        for i in 0..500 {
            match self.nation_statuses.get(&i) {
                Some(d) => w.write_u8(*d).unwrap(),
                _ => w.write_u8(0).unwrap(),
            }
        }
        if self.game_state == 1 {
            for i in 0..500 {
                match self.nation_teams.get(&i) {
                    Some(t) => w.write_u8(*t).unwrap(),
                    _ => w.write_u8(0).unwrap(),
                }
            }
            for i in 0..500 {
                match self.nation_discs.get(&i) {
                    Some(t) => {
                        w.write_u8(*t).unwrap();
                    }
                    _ => w.write_u8(0).unwrap(),
                }
            }
            w.write_u32::<LittleEndian>(0).unwrap();
            w.write_u8(0).unwrap();
            w.write_i32::<LittleEndian>(-1).unwrap();
        } else {
            for i in 0..500 {
                match self.turn_statuses.get(&i) {
                    Some(t) => w.write_u8(*t).unwrap(),
                    _ => w.write_u8(0).unwrap(),
                }
            }
            for i in 0..500 {
                match self.conn_statuses.get(&i) {
                    Some(t) => {
                        w.write_u8(*t).unwrap();
                    }
                    _ => w.write_u8(0).unwrap(),
                }
            }
            w.write_u32::<LittleEndian>(self.turn_number).unwrap();
            w.write_u8(0).unwrap();
            w.write_u32::<LittleEndian>(self.turnkey).unwrap();
        }
    }
}
