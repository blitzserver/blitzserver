use crate::num_derive::{FromPrimitive, ToPrimitive};
use crate::num_traits::{FromPrimitive, ToPrimitive};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{BufReader, Write};
use thiserror::Error;

use bincode::Options;
use std::collections::VecDeque;
use std::fs;

pub mod astralpacket_req;
pub mod astralpacket_resp;
pub mod disconnect_req;
pub mod disconnect_resp;
pub mod dmfile_req;
pub mod dmfile_resp;
pub mod gameinfo_req;
pub mod gameinfo_resp;
pub mod heartbeat_req;
pub mod loadingmessage_resp;
pub mod lobby;
pub mod map_req;
pub mod map_resp;
pub mod mapfile_req;
pub mod mapfile_resp;
pub mod mapimagefile_req;
pub mod mapimagefile_resp;
pub mod mapwinterfile_req;
pub mod mapwinterfile_resp;
pub mod modfile_req;
pub mod modfile_resp;
pub mod nationsselected_req;
pub mod pa_req;
pub mod pa_resp;
pub mod passwords_req;
pub mod passwords_resp;
pub mod setdisc_req;
pub mod setteam_req;
pub mod startgame_req;
pub mod submit2h_req;
pub mod submit2h_resp;
pub mod trn_req;
pub mod trn_resp;
pub mod twoh_req;
pub mod twoh_resp;
pub mod twohcrc_req;
pub mod twohcrc_resp;
pub mod uploadpretender_req;

pub use astralpacket_req::AstralPacketReq;
pub use disconnect_req::DisconnectReq;
pub use disconnect_resp::DisconnectResp;
pub use dmfile_resp::DmFileResp;
pub use gameinfo_req::GameInfoReq;
pub use gameinfo_resp::GameInfoResp;
pub use heartbeat_req::HeartbeatReq;
pub use loadingmessage_resp::LoadingMessageResp;
pub use map_req::MapReq;
pub use map_resp::MapResp;
pub use mapfile_req::MapFileReq;
pub use mapfile_resp::MapFileResp;
pub use mapimagefile_req::MapImageFileReq;
pub use mapimagefile_resp::MapImageFileResp;
pub use mapwinterfile_req::MapWinterFileReq;
pub use mapwinterfile_resp::MapWinterFileResp;
pub use modfile_req::ModFileReq;
pub use modfile_resp::ModFileResp;
pub use nationsselected_req::NationsSelectedReq;
pub use pa_req::PAReq;
pub use pa_resp::PAResp;
pub use passwords_req::PasswordsReq;
pub use passwords_resp::PasswordsResp;
pub use setdisc_req::SetDiscReq;
pub use setteam_req::SetTeamReq;
pub use startgame_req::StartGameReq;
pub use submit2h_req::Submit2hReq;
pub use submit2h_resp::Submit2hResp;
pub use trn_req::TrnReq;
pub use trn_resp::TrnResp;
pub use twoh_req::TwoHReq;
pub use twoh_resp::TwoHResp;
pub use twohcrc_req::TwoHCrcReq;
pub use twohcrc_resp::TwoHCrcResp;
pub use uploadpretender_req::UploadPretenderReq;

use self::astralpacket_resp::AstralPacketResp;
use self::dmfile_req::DmFileReq;
use self::lobby::lobbyinfo_resp::LobbyInfoResp;
use self::map_resp::{MapResp6, PlaneDef};

#[derive(Debug)]
pub enum Protocol {
    LOBBY,
    GAME,
}

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("No more data")]
    Disconnect(#[from] std::io::Error),
    #[error("Invalid header")]
    InvalidCompressionMethod(u8),
}

pub type PacketResult<T> = Result<T, PacketError>;

#[derive(Debug, Clone)]
pub struct Packet {
    pub header: Header,
    pub body: Body,
}

impl Packet {
    pub fn from_reader<R: std::io::Read>(
        r: &mut R,
        version: Option<u16>,
        protocol: Protocol,
    ) -> PacketResult<Packet> {
        let header = Header::from_reader(r)?;
        let mut body_buf: Vec<u8> = vec![0u8; header.length as usize];
        println!("Header is {:?}", header);
        if let Err(e) = r.read_exact(&mut body_buf) {
            println!("Warn; ran out of bytes? {:?}", e);
        }
        let mut reader = &body_buf[..];
        let body = if header.compression == CompressionType::Zlib {
            let _len = reader.read_u32::<LittleEndian>().unwrap();
            Body::from_reader(
                &mut std::io::BufReader::new(ZlibDecoder::new(reader)),
                version,
                protocol,
            )
        } else {
            Body::from_reader(&mut reader, version, protocol)
        };
        Ok(Packet { header, body })
    }
}

#[derive(Debug, Clone, PartialEq, ToPrimitive, FromPrimitive)]
enum CompressionType {
    Na = 0x48,
    Zlib = 0x4a,
}

#[derive(Debug, Clone)]
pub struct Header {
    unk: u8,
    compression: CompressionType,
    pub length: u32,
}

impl Header {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> PacketResult<Header> {
        let unk: u8 = r.read_u8()?;
        let compression_byte = r.read_u8()?;
        let compression = FromPrimitive::from_u8(compression_byte)
            .ok_or(PacketError::InvalidCompressionMethod(compression_byte))?;
        let length: u32 = r.read_u32::<LittleEndian>()?;
        Ok(Header {
            unk,
            compression,
            length,
        })
    }
    pub fn write<W: std::io::Write>(&self, w: &mut W) {
        w.write_u8(0x66).unwrap();
        w.write_u8(CompressionType::Zlib.to_u8().unwrap()).unwrap();
        w.write_u32::<LittleEndian>(self.length).unwrap();
    }
}

#[derive(Debug, Clone)]
pub enum Body {
    // Game
    HeartbeatReq(HeartbeatReq),
    UploadPretenderReq(UploadPretenderReq),
    AstralPacketReq(AstralPacketReq),
    AstralPacketResp(AstralPacketResp),
    GameInfoReq(GameInfoReq),
    GameInfoResp(GameInfoResp),
    DisconnectReq(DisconnectReq),
    StartGameReq(StartGameReq),
    NationsSelectedReq(NationsSelectedReq),
    PAResp(PAResp),
    LoadingMessageResp(LoadingMessageResp),
    PAReq(PAReq),
    PasswordsReq(PasswordsReq),
    TwoHCrcReq(TwoHCrcReq),
    PasswordsResp(PasswordsResp),
    TwoHCrcResp(TwoHCrcResp),
    TrnReq(TrnReq),
    TrnResp(TrnResp),
    MapReq(MapReq),
    MapResp(MapResp),
    MapResp6(MapResp6),
    MapFileReq(MapFileReq),
    MapFileResp(MapFileResp),
    MapImageFileReq(MapImageFileReq),
    MapImageFileResp(MapImageFileResp),
    MapWinterFileReq(MapWinterFileReq),
    MapWinterFileResp(MapWinterFileResp),
    Submit2hReq(Submit2hReq),
    Submit2hResp(Submit2hResp),
    TwoHReq(TwoHReq),
    TwoHResp(TwoHResp),
    DmFileReq(DmFileReq),
    DmFileResp(DmFileResp),
    ModFileReq(ModFileReq),
    ModFileResp(ModFileResp),
    SetTeamReq(SetTeamReq),
    SetDiscReq(SetDiscReq),

    // Lobby
    LobbyInfoResp(LobbyInfoResp),
}

impl Body {
    pub fn from_reader<R: std::io::BufRead>(
        r: &mut R,
        version: Option<u16>,
        protocol: Protocol,
    ) -> Body {
        match (protocol, version, r.read_u8().unwrap()) {
            (_, _, UploadPretenderReq::ID) => {
                Body::UploadPretenderReq(UploadPretenderReq::from_reader(r))
            }
            (_, _, HeartbeatReq::ID) => Body::HeartbeatReq(HeartbeatReq::from_reader(r)),
            (_, _, GameInfoReq::ID) => Body::GameInfoReq(GameInfoReq::from_reader(r)),
            (_, _, GameInfoResp::ID) => Body::GameInfoResp(GameInfoResp::from_reader(r)),
            (_, _, AstralPacketResp::ID) => {
                Body::AstralPacketResp(AstralPacketResp::from_reader(r, version))
            }
            (_, _, AstralPacketReq::ID) => Body::AstralPacketReq(AstralPacketReq::from_reader(r)),
            (_, _, StartGameReq::ID) => Body::StartGameReq(StartGameReq::from_reader(r)),
            (_, _, DisconnectReq::ID) => Body::DisconnectReq(DisconnectReq::from_reader(r)),
            (_, _, PAResp::ID) => Body::PAResp(PAResp::from_reader(r)),
            (_, _, PAReq::ID) => Body::PAReq(PAReq::from_reader(r)),
            (_, _, PasswordsReq::ID) => Body::PasswordsReq(PasswordsReq::from_reader(r)),
            (_, _, TwoHCrcReq::ID) => Body::TwoHCrcReq(TwoHCrcReq::from_reader(r)),
            (_, _, PasswordsResp::ID) => Body::PasswordsResp(PasswordsResp::from_reader(r)),
            (_, _, TwoHCrcResp::ID) => Body::TwoHCrcResp(TwoHCrcResp::from_reader(r)),
            (_, _, TrnReq::ID) => Body::TrnReq(TrnReq::from_reader(r)),
            (_, _, TrnResp::ID) => Body::TrnResp(TrnResp::from_reader(r)),
            (_, _, MapReq::ID) => Body::MapReq(MapReq::from_reader(r)),
            (_, Some(v), MapResp::ID) if v > 561 => Body::MapResp6(MapResp6::from_reader(r)),
            (_, _, MapFileReq::ID) => Body::MapFileReq(MapFileReq::from_reader(r)),
            (_, _, MapFileResp::ID) => Body::MapFileResp(MapFileResp::from_reader(r)),
            (_, v, MapImageFileReq::ID) => {
                Body::MapImageFileReq(MapImageFileReq::from_reader(r, v))
            }
            (_, _, MapImageFileResp::ID) => {
                Body::MapImageFileResp(MapImageFileResp::from_reader(r))
            }
            (_, _, MapWinterFileReq::ID) => {
                Body::MapWinterFileReq(MapWinterFileReq::from_reader(r))
            }
            (_, _, MapWinterFileResp::ID) => {
                Body::MapWinterFileResp(MapWinterFileResp::from_reader(r))
            }
            (_, _, Submit2hReq::ID) => Body::Submit2hReq(Submit2hReq::from_reader(r)),
            (_, _, Submit2hResp::ID) => Body::Submit2hResp(Submit2hResp::from_reader(r)),
            (_, _, TwoHReq::ID) => Body::TwoHReq(TwoHReq::from_reader(r)),
            (_, _, TwoHResp::ID) => Body::TwoHResp(TwoHResp::from_reader(r)),
            (_, _, DmFileReq::ID) => Body::DmFileReq(DmFileReq::from_reader(r, version)),
            (_, _, DmFileResp::ID) => Body::DmFileResp(DmFileResp::from_reader(r)),
            (_, _, ModFileReq::ID) => Body::ModFileReq(ModFileReq::from_reader(r)),
            (_, _, ModFileResp::ID) => Body::ModFileResp(ModFileResp::from_reader(r)),
            (Protocol::GAME, _, NationsSelectedReq::ID) => {
                Body::NationsSelectedReq(NationsSelectedReq::from_reader(r))
            }
            (Protocol::LOBBY, _, LobbyInfoResp::ID) => {
                Body::LobbyInfoResp(LobbyInfoResp::from_reader(r))
            }
            (_, _, SetTeamReq::ID) => Body::SetTeamReq(SetTeamReq::from_reader(r)),
            (_, _, SetDiscReq::ID) => Body::SetDiscReq(SetDiscReq::from_reader(r)),
            (_, _, d) => {
                let mut v = vec![];
                r.read_to_end(&mut v).unwrap();
                panic!(
                "What the fuck is that? What the FUCK is that? Mystery id {:x?}, full contents:\n{:x?}",
                d, v);
            }
        }
    }
    pub fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        match self {
            Self::LobbyInfoResp(p) => p.write(w, version),
            Self::NationsSelectedReq(p) => p.write(w, version),
            Self::PAResp(p) => p.write(w, version),
            Self::DisconnectReq(p) => p.write(w, version),
            Self::UploadPretenderReq(p) => p.write(w, version),
            Self::HeartbeatReq(p) => p.write(w, version),
            Self::AstralPacketReq(p) => p.write(w, version),
            Self::AstralPacketResp(p) => p.write(w, version),
            Self::GameInfoReq(p) => p.write(w, version),
            Self::GameInfoResp(p) => p.write(w, version),
            Self::StartGameReq(p) => p.write(w, version),
            Self::PAReq(p) => p.write(w, version),
            Self::PasswordsReq(p) => p.write(w, version),
            Self::TwoHCrcReq(p) => p.write(w, version),
            Self::PasswordsResp(p) => p.write(w, version),
            Self::TwoHCrcResp(p) => p.write(w, version),
            Self::LoadingMessageResp(p) => p.write(w, version),
            Self::TrnReq(p) => p.write(w, version),
            Self::TrnResp(p) => p.write(w, version),
            Self::MapReq(p) => p.write(w, version),
            Self::MapResp(p) => p.write(w, version),
            Self::MapResp6(p) => p.write(w, version),
            Self::MapFileReq(p) => p.write(w, version),
            Self::MapFileResp(p) => p.write(w, version),
            Self::MapImageFileReq(p) => p.write(w, version),
            Self::MapImageFileResp(p) => p.write(w, version),
            Self::MapWinterFileReq(p) => p.write(w, version),
            Self::MapWinterFileResp(p) => p.write(w, version),
            Self::Submit2hReq(p) => p.write(w, version),
            Self::Submit2hResp(p) => p.write(w, version),
            Self::TwoHReq(p) => p.write(w, version),
            Self::TwoHResp(p) => p.write(w, version),
            Self::DmFileReq(p) => p.write(w, version),
            Self::DmFileResp(p) => p.write(w, version),
            Self::ModFileReq(p) => p.write(w, version),
            Self::ModFileResp(p) => p.write(w, version),
            Self::SetTeamReq(p) => p.write(w, version),
            Self::SetDiscReq(p) => p.write(w, version),
        }
    }
}

pub trait BodyContents
where
    Self: std::fmt::Debug,
{
    const ID: u8;

    fn write<W: std::io::Write>(&self, w: &mut W, version: Option<u16>);

    fn write_packet<W: std::io::Write>(&self, w: &mut W, version: Option<u16>) {
        log::info!("                     <===  {:?}", self);
        let mut full: Vec<u8> = vec![];

        // Create body
        let mut body: Vec<u8> = vec![];
        self.write(&mut body, version);
        let mut packet_body: Vec<u8> = vec![];
        packet_body
            .write_u32::<LittleEndian>((body.len() + 5) as u32) // TODO: This cant be right
            .unwrap();

        let mut zlib = ZlibEncoder::new(packet_body, flate2::Compression::default());
        zlib.write_u8(Self::ID).unwrap();
        zlib.write_all(&body).unwrap();
        let packet_body = zlib.finish().unwrap();

        // Write header then body
        (Header {
            length: packet_body.len() as u32,
            compression: CompressionType::Zlib,
            unk: 0,
        })
        .write(&mut full);
        full.write_all(&packet_body[..]).unwrap();
        w.write_all(&full).unwrap();
    }
}

#[test]
fn deserialize_packet_game() {
    let mut openset = VecDeque::from([std::path::PathBuf::from("./test_data/packets/game")]);
    while let Some(path) = openset.pop_front() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    if !entry
                        .file_name()
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .starts_with("_")
                    {
                        openset.push_back(entry.path());
                    }
                }
                if ft.is_file() {
                    let mut file = std::fs::File::open(entry.path()).unwrap();
                    println!("Deserializing {:?}", entry.path());
                    let packet = Packet::from_reader(&mut file, Some(600), Protocol::GAME).unwrap();
                    println!("Packet {:?}", packet);
                }
            }
        }
    }
}

#[test]
fn deserialize_packet_lobby() {
    let mut openset = VecDeque::from([std::path::PathBuf::from("./test_data/packets/lobby")]);
    while let Some(path) = openset.pop_front() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    if !entry
                        .file_name()
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .starts_with("_")
                    {
                        openset.push_back(entry.path());
                    }
                }
                if ft.is_file() {
                    let mut file = std::fs::File::open(entry.path()).unwrap();
                    println!("Deserializing {:?}", entry.path());
                    let packet =
                        Packet::from_reader(&mut file, Some(600), Protocol::LOBBY).unwrap();
                    println!("Packet {:?}", packet);
                }
            }
        }
    }
}

#[test]
fn roundtrip_mapresp() {
    let packet = MapResp6 {
        planes: vec![
            PlaneDef {
                plane_name: "Pantokrator's Realm".to_string(),
                plane_file_name: "__randommap_asdasd.map".to_string(),
                map_len: 3244,
                map_hash: 1596100683,
                d6m_filename: "__randommap_asdasd.d6m".to_string(),
                img_len: 39911894,
                img_hash: 2075252347,
                plane_images: vec![],
            },
            PlaneDef {
                plane_name: "The Realm Beneath".to_string(),
                plane_file_name: "__under_asdasd.map".to_string(),
                map_len: 2887,
                map_hash: -1245367038,
                d6m_filename: "__under_asdasd.d6m".to_string(),
                img_len: 15728918,
                img_hash: -1793153284,
                plane_images: vec![],
            },
            PlaneDef {
                plane_name: "The Void".to_string(),
                plane_file_name: "nexus.map".to_string(),
                map_len: 491,
                map_hash: -1786016874,
                d6m_filename: "nexus.tga".to_owned(),
                img_len: 1862835,
                img_hash: 1491253869,
                plane_images: vec![],
            },
        ],
    };
    println!("Packet {:?}", packet);
    let mut bytes = vec![];
    packet.write_packet(&mut bytes, Some(600));
    println!("Res {:?}", bytes);
    let res = Packet::from_reader(&mut &bytes[..], Some(600), Protocol::GAME).unwrap();
    println!("Res {:?}", res);
}
