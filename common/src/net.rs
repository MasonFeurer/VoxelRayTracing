use std::borrow::Cow;
use crate::server::PlayerInfo;
use crate::world::{ChunkPos, Node, NodeAlloc, Voxel, VoxelPos};
use glam::Vec3;
use crate::resources::VoxelPack;

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
pub struct ChunksList(pub Vec<ChunkPos>);
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
    GetVoxelData(u32, VoxelPos),
    SetVoxel(VoxelPos, Voxel),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientCmd<'a> {
    HandshakeAccepted(Vec3, VoxelPack),
    HandshakeDenied,

    Kick(String),
    GivePlayersList(Vec<PlayerInfo>),
    GiveVoxelData(u32, VoxelPos, Voxel),
    GiveChunkData(ChunkPos, Cow<'a, [Node]>, NodeAlloc),
    GiveNewPos(Vec3),
}
