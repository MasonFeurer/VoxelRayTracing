/*
A library that holds items and functions needed by both the client and server.
*/

pub mod math;
pub mod net;
pub mod resources;
pub mod server;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Voxel(u16);
impl Voxel {
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn as_data(self) -> u16 {
        self.0
    }

    pub fn from_data(byte: u16) -> Self {
        Self(byte)
    }
}
