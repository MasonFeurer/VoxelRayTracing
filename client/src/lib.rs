/*
A library that provides the interface for creating and managing BlockWorld client.
Does not provide any graphics, just game-state.
*/

pub mod net;
pub mod player;
pub mod world;

pub use common;

use anyhow::Context;
use common::net::{ClientCmd, ConnError, ServerCmd};
use glam::{vec3, Vec3};
use net::ServerConn;
use player::Player;
use std::net::SocketAddr;
use world::ClientWorld;

pub struct GameState {
    pub user_name: String,
    pub player: Player,
    pub server_conn: Option<ServerConn>,
    pub world: ClientWorld,
}
impl GameState {
    pub fn new(user_name: String, world: ClientWorld, player_pos: Vec3) -> Self {
        Self {
            user_name,
            player: Player::new(player_pos, 0.2),
            server_conn: None,
            world,
        }
    }
}
impl GameState {}
/// Server functions
impl GameState {
    pub fn send_cmd(&mut self, cmd: ServerCmd) -> anyhow::Result<()> {
        self.server_conn
            .as_mut()
            .ok_or(ConnError::NoServer)?
            .write(cmd)
    }
    pub fn recv_cmd(&mut self) -> anyhow::Result<ClientCmd> {
        self.server_conn.as_mut().ok_or(ConnError::NoServer)?.read()
    }
    pub fn try_recv_cmd(&mut self) -> anyhow::Result<Option<ClientCmd>> {
        self.server_conn
            .as_mut()
            .ok_or(ConnError::NoServer)?
            .try_read()
    }

    pub fn disconnect(&mut self) -> anyhow::Result<()> {
        let Some(server) = &mut self.server_conn else {
            return Ok(());
        };
        _ = server
            .write(ServerCmd::DisconnectNotice)
            .context("Failed to send DisconnectNotice");
        self.server_conn = None;
        Ok(())
    }

    pub fn join_server(&mut self, addr: SocketAddr) -> anyhow::Result<()> {
        let conn =
            ServerConn::establish(addr, self.user_name.clone()).context("Failed to join server")?;
        self.server_conn = Some(conn);
        Ok(())
    }
}
