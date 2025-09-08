use crate::server::PlayerInfo;
use crate::world::{Node, Voxel};
use glam::IVec3;

impl std::error::Error for ConnError {}
#[derive(Debug)]
pub enum ConnError {
    NoServer,
    ServerDeniedConnection,
    ServerGaveInvalidData,
    ClientGaveInvalidData,
}
impl std::fmt::Display for ConnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerCmd {
    Handshake { name: String },

    DisconnectNotice,
    GetPlayersList,
    GetVoxelData(u32, IVec3),
    GetChunkData(u32, IVec3),
    PlaceVoxelData(Voxel, IVec3),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientCmd {
    HandshakeAccepted,
    HandshakeDenied,

    Kick(String),
    PlayersList(Vec<PlayerInfo>),
    GiveVoxelData(u32, IVec3, Voxel),
    GiveChunkData(u32, IVec3, Vec<Node>),
}
