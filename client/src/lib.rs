/*
A library that provides the interface for creating and managing BlockWorld client.
Does not provide any graphics, just game-state.
*/

pub mod net;

pub use common;

use anyhow::Context;
use common::net::{ClientCmd, ConnError, ServerCmd};
use net::ServerConn;
use std::net::SocketAddr;

pub struct GameState {
    pub user_name: String,
    pub server_conn: Option<ServerConn>,
}
impl GameState {
    pub fn new(user_name: String) -> Self {
        Self {
            user_name,
            server_conn: None,
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
