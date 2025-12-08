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
use common::world::{NodeAddr, SetVoxelErr};
use glam::{IVec3, Vec3};
use net::ServerConn;
use player::Player;
use std::net::SocketAddr;
use std::time::SystemTime;
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

#[derive(Default)]
pub struct CmdResult {
    pub kicked: bool,
    pub updated_chunks: Vec<(IVec3, NodeAddr, usize)>,
    pub received_oob_chunks: Vec<IVec3>,
}

/// Server functions
impl GameState {
    pub fn _send_cmd(&mut self, cmd: ServerCmd) -> anyhow::Result<()> {
        self.server_conn
            .as_mut()
            .ok_or(ConnError::NoServer)?
            .write(cmd)
    }
    pub fn _recv_cmd(&mut self) -> anyhow::Result<ClientCmd> {
        self.server_conn.as_mut().ok_or(ConnError::NoServer)?.read()
    }
    pub fn _try_recv_cmd(&mut self) -> anyhow::Result<Option<ClientCmd>> {
        self.server_conn
            .as_mut()
            .ok_or(ConnError::NoServer)?
            .try_read()
    }

    pub fn process_cmd(&mut self, cmd: ClientCmd, rs: &mut CmdResult) {
        match cmd {
            ClientCmd::GiveChunkData(pos, nodes, _node_alloc) => {
                match self.world.create_chunk(pos, &nodes) {
                    Ok(addr) => rs.updated_chunks.push((pos, addr, nodes.len())),
                    Err(SetVoxelErr::PosOutOfBounds) => rs.received_oob_chunks.push(pos),
                    Err(err) => println!("Encountered error creating chunk: {err:?}"),
                };
            }
            ClientCmd::Kick(reason) => {
                rs.kicked = true;
                println!("We've been kicked : {reason:?}");
            }
            ClientCmd::PlayersList(_list) => {}
            _ => {}
        }
    }
    // will process commands from the server until the given timeout duration has passed
    pub fn process_cmds_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> anyhow::Result<CmdResult> {
        let start_time = SystemTime::now();

        let mut rs = CmdResult::default();

        let mut read = self
            .server_conn
            .as_mut()
            .ok_or(ConnError::NoServer)?
            .try_read();
        while let Some(cmd) = read? {
            if SystemTime::now().duration_since(start_time).unwrap() >= timeout {
                break;
            }
            self.process_cmd(cmd, &mut rs);
            read = self.server_conn.as_mut().unwrap().try_read();
        }
        Ok(rs)
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
