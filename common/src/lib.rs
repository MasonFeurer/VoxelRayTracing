/*
A library that holds items and functions needed by both the client and server.
*/

pub mod math;
pub mod net;
pub mod resources;
pub mod server;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Voxel(u8);
impl Voxel {
    pub fn as_data(self) -> u8 {
        self.0
    }

    pub fn from_data(byte: u8) -> Self {
        Self(byte)
    }
}
