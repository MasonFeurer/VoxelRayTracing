/*
A library that provides an interface for creating and managing a BlockWorld server-world.
*/

pub mod net;
pub mod world;

use std::borrow::Cow;
pub use common;

use common::net::{ClientCmd, ServerCmd};
use common::server::PlayerInfo;
use common::world::{world_to_chunk_pos, Node, NodeAlloc, NODES_PER_CHUNK};
use glam::{vec3, IVec3, Vec3};
use net::ClientConn;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc};
use world::gen::{BuiltFeature, WorldGen};
use world::{ServerChunk, ServerWorld};

use common::log::*;
use crate::world::WorldFsExt;

pub type ClientId = u64;

pub struct Client {
    pub name: String,
    pub conn: ClientConn,
    pub pos: Vec3,
    pub render_distance: u32,

    pub wants_chunks: HashSet<IVec3>,
}
impl Client {
    pub fn new(name: String, conn: ClientConn) -> Self {
        Self {
            name,
            conn,
            pos: Vec3::ZERO,
            render_distance: 0,
            wants_chunks: Default::default(),
        }
    }

    pub fn using_chunk(&self, pos: IVec3) -> bool {
        self.wants_chunks.contains(&pos)
    }

    pub fn address(&self) -> SocketAddr {
        self.conn.stream.local_addr().unwrap()
    }

    pub fn send_cmd(&mut self, cmd: ClientCmd) {
        if let Err(err) = self.conn.write(cmd)
        {
            warn!("Failed to send command to client {:?} : {:?}", self.name, err)
        }
    }
}

#[derive(Clone, Copy)]
pub struct DirtyChunk {
    pub pos: IVec3,
    pub source: Option<ClientId>
}

pub struct ChunkBuilder {
    done: Arc<AtomicBool>,
}
impl ChunkBuilder {
    pub fn spawn<Fs: WorldFsExt + Sync + Send + 'static>(
        gen: Arc<WorldGen>,
        chunks: Vec<IVec3>,
        send: Sender<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
        fs: Arc<Fs>
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let done_copy = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut built_features = Vec::new();
            let mut node_buffer = vec![Node::ZERO; NODES_PER_CHUNK as usize];
            for pos in chunks {
                let chunk = if let Some(chunk) = fs.read_chunk(pos) {
                    chunk
                } else {
                    gen.generate_chunk(&mut node_buffer, pos, &mut built_features)
                };
                send.send((pos, chunk, built_features.clone())).unwrap();
                built_features.clear();
                node_buffer.fill(Node::ZERO);
            }
            done_copy.store(true, Ordering::Relaxed);
        });
        Self { done }
    }

    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }
}

pub fn connect_clients_blocking(
    listener: TcpListener,
    sender: Sender<Client>,
    kill: Arc<AtomicBool>,
    client_start_pos: Vec3,
) {
    for stream in listener.incoming() {
        if kill.load(Ordering::Relaxed) {
            break;
        }
        match stream {
            Ok(stream) => {
                info!(
                    "Establishing client connection from {:?}",
                    stream.local_addr()
                );
                match ClientConn::establish(stream, client_start_pos) {
                    Ok((conn, name)) => {
                        info!("Connected client: {name}!");
                        _ = sender.send(Client::new(name, conn));
                    }
                    Err(err) => warn!("Failed to establish client connection: {err:?}")
                };
            }
            Err(err) => warn!("Failed to listen on TCP: {err:?}")
        }
    }
}

pub struct ServerState {
    pub address: SocketAddr,
    pub name: String,
    pub clients: HashMap<ClientId, Client>,
    pub new_clients_recv: Option<Receiver<Client>>,

    pub world: ServerWorld,

    pub chunk_builder_send: Sender<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
    pub chunk_builder_recv: Receiver<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
    pub chunks_to_build: Vec<IVec3>,
    pub chunk_builders: Vec<ChunkBuilder>,
    pub dirty_chunks: HashMap<IVec3, Option<ClientId>>,

    pub kill: Arc<AtomicBool>,
}
impl ServerState {
    pub fn new(address: SocketAddr, name: String, world: ServerWorld) -> Self {
        let (chunk_builder_send, chunk_builder_recv) = channel();
        Self {
            address,
            name,
            clients: HashMap::new(),
            new_clients_recv: None,

            world,

            chunk_builder_send,
            chunk_builder_recv,
            chunks_to_build: Default::default(),
            chunk_builders: vec![],
            dirty_chunks: HashMap::default(),

            kill: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_player_list(&self) -> Vec<PlayerInfo> {
        self.clients
            .iter()
            .map(|(_id, client)| PlayerInfo {
                name: client.name.clone(),
                pos: client.pos,
            })
            .collect()
    }

    pub fn stop(&mut self) {
        self.kill.store(true, Ordering::Relaxed);
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.address)?;

        let (sender, receiver) = channel();
        self.new_clients_recv = Some(receiver);
        let kill = Arc::clone(&self.kill);

        let client_start_y = self.world.gen.terrain_h_at(0, 0) as f32 + 10.0;
        let client_start_pos = vec3(0.0, client_start_y, 0.0);

        std::thread::spawn(move || {
            connect_clients_blocking(listener, sender, kill, client_start_pos)
        });
        Ok(())
    }

    pub fn update<Fs: WorldFsExt + Sync + Send + 'static>(&mut self, fs: Arc<Fs>) {
        // --- Add any new clients to the client list ---
        if let Some(new_clients) = &self.new_clients_recv {
            while let Ok(client) = new_clients.try_recv() {
                self.clients.insert(fastrand::u64(..), client);
            }
        }
        // --- Remove any clients that have silently disconnected ---
        self.clients.retain(|_, client| !client.conn.broken_pipe);

        // --- Use any generated chunks from the chunk builders ---
        while let Ok((pos, chunk, built_features)) = self.chunk_builder_recv.try_recv() {
            self.world.chunks.insert(pos, chunk);
            self.dirty_chunks.insert(pos, None);
            self.world.unplaced_features.extend(built_features);
        }

        // --- Send any updated chunks to all the clients ---
        for (chunk_pos, source) in &self.dirty_chunks {
            let chunk = self.world.get_chunk(*chunk_pos).unwrap();
            let nodes = chunk.used_nodes();

            for (client_id, client) in &mut self.clients {
                if Some(*client_id) == *source {
                    continue;
                }
                // If the chunk has not been requested by the client, or if the client has disconnected,
                // don't bother sending it.
                if client.conn.broken_pipe || !client.using_chunk(*chunk_pos) {
                    continue;
                }
                client.send_cmd(ClientCmd::GiveChunkData(
                    *chunk_pos,
                    Cow::Borrowed(nodes),
                    NodeAlloc::new(0..1, 1..2),
                ));
            }
        }
        self.dirty_chunks.clear();

        // --- Spawn builders for world generation ---
        // remove finished chunk builders
        for idx in (0..self.chunk_builders.len()).rev() {
            if self.chunk_builders[idx].is_done() {
                _ = self.chunk_builders.remove(idx);
            }
        }
        let mut chunks_to_build = self.chunks_to_build.iter().copied();

        let chunks_per_buider = 128;
        while self.chunk_builders.len() < 16 {
            let next: Vec<_> = (&mut chunks_to_build).take(chunks_per_buider).collect();
            if next.is_empty() {
                break;
            }
            self.chunk_builders.push(ChunkBuilder::spawn(
                Arc::clone(&self.world.gen),
                next,
                self.chunk_builder_send.clone(),
                Arc::clone(&fs),
            ));
        }
        self.chunks_to_build = chunks_to_build.collect();
    }

    pub fn update_world(&mut self) {
        for chunk_pos in self.world.place_features() {
            self.dirty_chunks.insert(chunk_pos, None);
        }
    }

    fn handle_client_cmd(&mut self, client_id: ClientId, cmd: ServerCmd, player_list: &[PlayerInfo]) {
        let client = self.clients.get_mut(&client_id).unwrap();

        match cmd {
            ServerCmd::Handshake { .. } => {}
            ServerCmd::DisconnectNotice => {
                client.conn.broken_pipe = true;
                info!("Received disconnect notice from client {:?}", client.name);
            }
            ServerCmd::GetPlayersList => {
                client.send_cmd(ClientCmd::PlayersList(player_list.to_vec()));
            }
            ServerCmd::UpdateMyPlayerPos(new_pos) => {
                client.pos = new_pos;
            }
            ServerCmd::UpdateMyRenderDistance(new_dist) => {
                client.render_distance = new_dist;
            }
            ServerCmd::LoadChunks(chunks) => {
                for chunk_pos in chunks.0 {
                    if client.conn.broken_pipe {
                        break;
                    }
                    client.wants_chunks.insert(chunk_pos);
                    if let Some(data) = self.world.get_chunk(chunk_pos) {
                        client.send_cmd(ClientCmd::GiveChunkData(
                            chunk_pos,
                            Cow::Borrowed(data.used_nodes()),
                            NodeAlloc::new(0..1, 1..2),
                        ));
                    } else {
                        if !self.chunks_to_build.contains(&chunk_pos) {
                            self.chunks_to_build.push(chunk_pos);
                        }
                    }
                }
            }
            ServerCmd::UnloadChunks(chunks) => {
                for chunk_pos in chunks.0 {
                    _ = client.wants_chunks.remove(&chunk_pos);
                }
            }
            ServerCmd::GetVoxelData(_id, _pos) => {}
            ServerCmd::SetVoxel(pos, voxel) => {
                if let Err(err) = self.world.set_voxel(pos, voxel) {
                    warn!("Failed to set voxel (from client) at {:?} : {:?}", pos, err);
                }
                info!("Received `SetVoxel` command from client {:?} : {:?} = {:?}", client.name, pos, voxel);
                self.dirty_chunks.insert(world_to_chunk_pos(pos), Some(client_id));
            }
        }
    }

    pub fn handle_clients(&mut self) {
        let player_list = self.get_player_list();
        let poll = poll_clients(self.clients.iter_mut());
        for (client_idx, cmd) in poll.commands {
            self.handle_client_cmd(client_idx, cmd, &player_list);
        }

        for client_idx in poll.remove_clients.into_iter().rev() {
            _ = self.clients.remove(&client_idx);
        }
    }
}

pub struct PollResults {
    pub remove_clients: Vec<ClientId>,
    pub commands: Vec<(ClientId, ServerCmd)>,
}
pub fn poll_clients<'a>(clients: impl Iterator<Item = (&'a ClientId, &'a mut Client)>) -> PollResults {
    let mut remove_clients = vec![];
    let mut commands = vec![];

    for (client_id, client) in clients {
        let rs = client.conn.try_read();

        let cmd = match rs {
            Ok(Some(cmd)) => cmd,
            Ok(None) => continue,
            Err(err) => {
                warn!("Failed to poll commands from client {:?} : {:?}", client.name, err);
                remove_clients.push(*client_id);
                continue;
            }
        };
        commands.push((*client_id, cmd));
    }
    PollResults {
        remove_clients,
        commands,
    }
}
