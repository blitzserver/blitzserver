use crate::num_derive::{FromPrimitive, ToPrimitive};
use crate::num_traits::{FromPrimitive, ToPrimitive};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::Write;
use thiserror::Error;

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
pub mod pa_resp;
pub mod passwords_req;
pub mod passwords_resp;
pub mod startgame_req;
pub mod submit2h_req;
pub mod submit2h_resp;
pub mod trn_req;
pub mod trn_resp;
pub mod twoh_req;
pub mod twoh_resp;
pub mod twohcrc_req;
pub mod twohcrc_resp;
pub mod pa_req;
pub mod uploadpretender_req;
pub mod nationsselected_req;
pub mod setteam_req;
pub mod setdisc_req;

pub use astralpacket_req::AstralPacketReq;
pub use astralpacket_resp::AstralPacketResp;
pub use disconnect_req::DisconnectReq;
pub use disconnect_resp::DisconnectResp;
pub use dmfile_req::DmFileReq;
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
pub use pa_resp::PAResp;
pub use passwords_req::PasswordsReq;
pub use passwords_resp::PasswordsResp;
pub use startgame_req::StartGameReq;
pub use submit2h_req::Submit2hReq;
pub use submit2h_resp::Submit2hResp;
pub use trn_req::TrnReq;
pub use trn_resp::TrnResp;
pub use twoh_req::TwoHReq;
pub use twoh_resp::TwoHResp;
pub use twohcrc_req::TwoHCrcReq;
pub use twohcrc_resp::TwoHCrcResp;
pub use pa_req::PAReq;
pub use uploadpretender_req::UploadPretenderReq;
pub use setteam_req::SetTeamReq;
pub use setdisc_req::SetDiscReq;

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("No more data")]
    Disconnect(#[from] std::io::Error),
    #[error("Invalid header")]
    InvalidCompressionMethod(u8)
}

pub type PacketResult<T> = Result<T, PacketError>;

#[derive(Debug, Clone)]
pub struct Packet {
    pub header: Header,
    pub body: Body,
}

impl Packet {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> PacketResult<Packet> {
        let header = Header::from_reader(r)?;
        let mut body_buf: Vec<u8> = vec![0u8; header.length as usize];
        r.read_exact(&mut body_buf).unwrap();
        let mut reader = &body_buf[..];
        let body = if header.compression == CompressionType::Zlib {
            let _len = reader.read_u32::<LittleEndian>().unwrap();
            Body::from_reader(&mut std::io::BufReader::new(ZlibDecoder::new(reader)))
        } else {
            Body::from_reader(&mut reader)
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
        let compression = FromPrimitive::from_u8(compression_byte).ok_or(PacketError::InvalidCompressionMethod(compression_byte))?;
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
}

impl Body {
    pub fn from_reader<R: std::io::BufRead>(r: &mut R) -> Body {
        match r.read_u8().unwrap() {
            UploadPretenderReq::ID => Body::UploadPretenderReq(UploadPretenderReq::from_reader(r)),
            HeartbeatReq::ID => Body::HeartbeatReq(HeartbeatReq::from_reader(r)),
            GameInfoReq::ID => Body::GameInfoReq(GameInfoReq::from_reader(r)),
            GameInfoResp::ID => Body::GameInfoResp(GameInfoResp::from_reader(r)),
            AstralPacketResp::ID => Body::AstralPacketResp(AstralPacketResp::from_reader(r)),
            AstralPacketReq::ID => Body::AstralPacketReq(AstralPacketReq::from_reader(r)),
            StartGameReq::ID => Body::StartGameReq(StartGameReq::from_reader(r)),
            DisconnectReq::ID => Body::DisconnectReq(DisconnectReq::from_reader(r)),
            PAResp::ID => Body::PAResp(PAResp::from_reader(r)),
            PAReq::ID => Body::PAReq(PAReq::from_reader(r)),
            PasswordsReq::ID => Body::PasswordsReq(PasswordsReq::from_reader(r)),
            TwoHCrcReq::ID => Body::TwoHCrcReq(TwoHCrcReq::from_reader(r)),
            PasswordsResp::ID => Body::PasswordsResp(PasswordsResp::from_reader(r)),
            TwoHCrcResp::ID => Body::TwoHCrcResp(TwoHCrcResp::from_reader(r)),
            TrnReq::ID => Body::TrnReq(TrnReq::from_reader(r)),
            TrnResp::ID => Body::TrnResp(TrnResp::from_reader(r)),
            MapReq::ID => Body::MapReq(MapReq::from_reader(r)),
            MapResp::ID => Body::MapResp(MapResp::from_reader(r)),
            MapFileReq::ID => Body::MapFileReq(MapFileReq::from_reader(r)),
            MapFileResp::ID => Body::MapFileResp(MapFileResp::from_reader(r)),
            MapImageFileReq::ID => Body::MapImageFileReq(MapImageFileReq::from_reader(r)),
            MapImageFileResp::ID => Body::MapImageFileResp(MapImageFileResp::from_reader(r)),
            MapWinterFileReq::ID => Body::MapWinterFileReq(MapWinterFileReq::from_reader(r)),
            MapWinterFileResp::ID => Body::MapWinterFileResp(MapWinterFileResp::from_reader(r)),
            Submit2hReq::ID => Body::Submit2hReq(Submit2hReq::from_reader(r)),
            Submit2hResp::ID => Body::Submit2hResp(Submit2hResp::from_reader(r)),
            TwoHReq::ID => Body::TwoHReq(TwoHReq::from_reader(r)),
            TwoHResp::ID => Body::TwoHResp(TwoHResp::from_reader(r)),
            DmFileReq::ID => Body::DmFileReq(DmFileReq::from_reader(r)),
            DmFileResp::ID => Body::DmFileResp(DmFileResp::from_reader(r)),
            ModFileReq::ID => Body::ModFileReq(ModFileReq::from_reader(r)),
            ModFileResp::ID => Body::ModFileResp(ModFileResp::from_reader(r)),
            NationsSelectedReq::ID => Body::NationsSelectedReq(NationsSelectedReq::from_reader(r)),
            SetTeamReq::ID => Body::SetTeamReq(SetTeamReq::from_reader(r)),
            SetDiscReq::ID => Body::SetDiscReq(SetDiscReq::from_reader(r)),

            d => {
                let mut v = vec![];
                r.read_to_end(&mut v).unwrap();
                panic!(
                "What the fuck is that? What the FUCK is that? Mystery id {:x?}, full contents:\n{:x?}",
                d, v);
            }
        }
    }
    pub fn write<W: std::io::Write>(&self, w: &mut W) {
        match self {
            Self::NationsSelectedReq(p) => p.write(w),
            Self::PAResp(p) => p.write(w),
            Self::DisconnectReq(p) => p.write(w),
            Self::UploadPretenderReq(p) => p.write(w),
            Self::HeartbeatReq(p) => p.write(w),
            Self::AstralPacketReq(p) => p.write(w),
            Self::AstralPacketResp(p) => p.write(w),
            Self::GameInfoReq(p) => p.write(w),
            Self::GameInfoResp(p) => p.write(w),
            Self::StartGameReq(p) => p.write(w),
            Self::PAReq(p) => p.write(w),
            Self::PasswordsReq(p) => p.write(w),
            Self::TwoHCrcReq(p) => p.write(w),
            Self::PasswordsResp(p) => p.write(w),
            Self::TwoHCrcResp(p) => p.write(w),
            Self::LoadingMessageResp(p) => p.write(w),
            Self::TrnReq(p) => p.write(w),
            Self::TrnResp(p) => p.write(w),
            Self::MapReq(p) => p.write(w),
            Self::MapResp(p) => p.write(w),
            Self::MapFileReq(p) => p.write(w),
            Self::MapFileResp(p) => p.write(w),
            Self::MapImageFileReq(p) => p.write(w),
            Self::MapImageFileResp(p) => p.write(w),
            Self::MapWinterFileReq(p) => p.write(w),
            Self::MapWinterFileResp(p) => p.write(w),
            Self::Submit2hReq(p) => p.write(w),
            Self::Submit2hResp(p) => p.write(w),
            Self::TwoHReq(p) => p.write(w),
            Self::TwoHResp(p) => p.write(w),
            Self::DmFileReq(p) => p.write(w),
            Self::DmFileResp(p) => p.write(w),
            Self::ModFileReq(p) => p.write(w),
            Self::ModFileResp(p) => p.write(w),
            Self::SetTeamReq(p) => p.write(w),
            Self::SetDiscReq(p) => p.write(w),
        }
    }
}

pub trait BodyContents
where
    Self: std::fmt::Debug,
{
    const ID: u8;

    fn write<W: std::io::Write>(&self, w: &mut W);

    fn write_packet<W: std::io::Write>(&self, w: &mut W) {
        log::debug!("                     <===  {:?}", self);
        let mut full: Vec<u8> = vec![];

        // Create body
        let mut body: Vec<u8> = vec![];
        self.write(&mut body);
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
