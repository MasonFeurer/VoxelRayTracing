/*
A library that provides the interface for creating and managing BlockWorld client.
Does not provide any graphics, just game-state.
*/

pub mod net;
pub mod player;
pub mod world;

pub use common;

use anyhow::Context;
use common::net::{ChunksList, ClientCmd, ServerCmd};
use common::world::{NodeAddr, SetVoxelErr};
use glam::IVec3;
use net::ServerConn;
use player::Player;
use std::collections::HashSet;
use std::time::SystemTime;
use world::ClientWorld;

#[derive(Default)]
pub struct CmdResult {
    pub kicked: bool,
    pub updated_chunks: Vec<(IVec3, NodeAddr, usize)>,
    pub received_oob_chunks: Vec<IVec3>,
}

pub struct GameState {
    pub user_name: String,
    pub player: Player,
    pub world: ClientWorld,

    host: ServerConn,
    chunk_requests_sent: HashSet<IVec3>,
}
impl GameState {
    pub fn new(user_name: String, world: ClientWorld, server_conn: ServerConn) -> Self {
        Self {
            user_name,
            player: Player::new(server_conn.player_pos, 0.2),
            world,

            host: server_conn,
            chunk_requests_sent: Default::default(),
        }
    }
}
/// World functions
impl GameState {
    pub fn center_chunks(&mut self, anchor: IVec3) {
        let removed_chunks = self.world.center_chunks(anchor);
        let (posis, chunks): (Vec<_>, Vec<_>) = removed_chunks.into_iter().unzip();
        for chunk in chunks {
            _ = self.world.free_chunk(chunk);
        }
        if posis.len() > 0 {
            _ = self.host.write(ServerCmd::UnloadChunks(ChunksList(posis)));
        }
    }
}
/// Server functions
impl GameState {
    pub fn request_missing_chunks(&mut self) {
        let mut empty_chunks = self.world.empty_chunks().collect::<Vec<_>>();
        empty_chunks.sort_by(|a, b| {
            let center = self.player.pos;
            let a_dist = center.distance(a.global_center(&self.world).as_vec3());
            let b_dist = center.distance(b.global_center(&self.world).as_vec3());
            a_dist
                .partial_cmp(&b_dist)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut chunks_to_load: Vec<IVec3> = vec![];
        for chunk in empty_chunks {
            let global_pos = chunk.global_pos(&self.world);
            if self.chunk_requests_sent.contains(&global_pos) {
                continue;
            }
            chunks_to_load.push(global_pos);
        }
        if !chunks_to_load.is_empty() {
            if let Err(err) = self
                .host
                .write(ServerCmd::LoadChunks(ChunksList(chunks_to_load.clone())))
            {
                println!("Failed to send cmd to server: {err:?}");
            } else {
                self.chunk_requests_sent.extend(chunks_to_load);
            }
        }
    }

    pub fn process_cmd(&mut self, cmd: ClientCmd, rs: &mut CmdResult) {
        match cmd {
            ClientCmd::GiveChunkData(pos, nodes, _node_alloc) => {
                self.chunk_requests_sent.remove(&pos);
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

        let mut read = self.host.try_read();
        while let Some(cmd) = read? {
            if SystemTime::now().duration_since(start_time).unwrap() >= timeout {
                break;
            }
            self.process_cmd(cmd, &mut rs);
            read = self.host.try_read();
        }
        Ok(rs)
    }

    pub fn disconnect(&mut self) -> anyhow::Result<()> {
        _ = self
            .host
            .write(ServerCmd::DisconnectNotice)
            .context("Failed to send DisconnectNotice");
        Ok(())
    }
}
