use crate::server::PlayerInfo;
use crate::world::{Node, NodeAlloc, Voxel};
use glam::{IVec3, Vec3};

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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChunksList(pub Vec<IVec3>);
impl std::fmt::Debug for ChunksList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0.len(), f)?;
        f.write_str(" chunks")
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerCmd {
    Handshake { name: String },

    UpdateMyPlayerPos(Vec3),
    UpdateMyRenderDistance(u32),
    LoadChunks(ChunksList),
    UnloadChunks(ChunksList),

    DisconnectNotice,
    GetPlayersList,
    GetVoxelData(u32, IVec3),
    PlaceVoxelData(Voxel, IVec3),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientCmd {
    HandshakeAccepted,
    HandshakeDenied,

    Kick(String),
    PlayersList(Vec<PlayerInfo>),
    GiveVoxelData(u32, IVec3, Voxel),
    GiveChunkData(IVec3, Vec<Node>, NodeAlloc),
}
