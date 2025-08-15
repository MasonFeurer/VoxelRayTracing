use crate::Voxel;
use glam::{IVec3, Vec3};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub pos: Vec3,
}

pub trait Server {
    fn name(&self) -> &str;
    fn address(&self) -> String;
    fn get_player_list(&self) -> Vec<PlayerInfo>;

    fn set_voxel(&mut self, pos: IVec3, v: Voxel);
    fn get_voxel(&self, pos: IVec3) -> Voxel;
    fn get_chunk(&self, pos: IVec3) -> Vec<Voxel>;
}
