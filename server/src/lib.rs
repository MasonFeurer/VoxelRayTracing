/*
A library that provides an interface for creating and managing a BlockWorld server-world.
*/

pub mod net;
pub mod world;

pub use common;

use common::net::{ClientCmd, ServerCmd};
use common::resources::{loader, VoxelPack, WorldFeatures, WorldPreset};
use common::server::PlayerInfo;
use common::world::{Node, NodeAlloc, NODES_PER_CHUNK};
use glam::{vec3, IVec3, Vec3};
use net::ClientConn;
use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use world::gen::{BuiltFeature, WorldGen};
use world::{ServerChunk, ServerWorld};

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
}

pub struct Resources {
    pub voxelpack: VoxelPack,
    pub world_features: WorldFeatures,
    pub world_presets: Vec<WorldPreset>,
}
impl Resources {
    pub fn load(dir: &String) -> anyhow::Result<Self> {
        let voxelpack = std::fs::read_to_string(format!("{dir}/voxelpack.ron"))?;
        let voxelpack = loader::parse_voxelpack(&voxelpack)?;

        let world_features = std::fs::read_to_string(format!("{dir}/features.ron"))?;
        let world_features = loader::parse_world_features(&world_features, &voxelpack)?;

        let world_presets = std::fs::read_to_string(format!("{dir}/worldpresets.ron"))?;
        let world_presets =
            loader::parse_world_presets(&world_presets, &voxelpack, &world_features)?;

        Ok(Self {
            voxelpack,
            world_features,
            world_presets,
        })
    }
}

pub struct ChunkBuilder {
    done: Arc<AtomicBool>,
}
impl ChunkBuilder {
    pub fn spawn(
        gen: Arc<WorldGen>,
        chunks: Vec<IVec3>,
        send: Sender<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let done_copy = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut built_features = Vec::new();
            let mut node_buffer = vec![Node::ZERO; NODES_PER_CHUNK as usize];
            for pos in chunks {
                let chunk = gen.generate_chunk(&mut node_buffer, pos, &mut built_features);
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
            println!("Stopping client connections; Dropping TCPListener...");
            break;
        }
        match stream {
            Ok(stream) => {
                println!(
                    "Establishing client connection from {:?}",
                    stream.local_addr()
                );
                match ClientConn::establish(stream, client_start_pos) {
                    Ok((conn, name)) => {
                        println!("Connected client: {name}!");
                        _ = sender.send(Client::new(name, conn));
                    }
                    Err(err) => println!("Failed to establish client connection: {err:?}"),
                };
            }
            Err(err) => println!("Failed to receive client stream from TcpListener : {err:?}"),
        }
    }
}

pub struct ServerState {
    pub address: SocketAddr,
    pub name: String,
    pub clients: Vec<Client>,
    pub new_clients_recv: Option<Receiver<Client>>,

    pub world: ServerWorld,

    pub chunk_builder_send: Sender<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
    pub chunk_builder_recv: Receiver<(IVec3, ServerChunk, Vec<BuiltFeature>)>,
    pub chunks_to_build: Vec<IVec3>,
    pub chunks_to_build_set: HashSet<IVec3>,
    pub chunk_builders: Vec<ChunkBuilder>,
    pub dirty_chunks: HashSet<IVec3>,

    pub kill: Arc<AtomicBool>,
}
impl ServerState {
    pub fn new(address: SocketAddr, name: String, world: ServerWorld) -> Self {
        let (chunk_builder_send, chunk_builder_recv) = channel();
        Self {
            address,
            name,
            clients: vec![],
            new_clients_recv: None,

            world,

            chunk_builder_send,
            chunk_builder_recv,
            chunks_to_build: Default::default(),
            chunks_to_build_set: Default::default(),
            chunk_builders: vec![],
            dirty_chunks: HashSet::default(),

            kill: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn player_list(&self) -> Vec<PlayerInfo> {
        self.clients
            .iter()
            .map(|client| PlayerInfo {
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

    pub fn update(&mut self) {
        // --- Add any new clients to the client list ---
        if let Some(new_clients) = &self.new_clients_recv {
            while let Ok(client) = new_clients.try_recv() {
                self.clients.push(client);
            }
        }
        // --- Remove any clients that have silently disconnected ---
        for idx in (0..self.clients.len()).rev() {
            if self.clients[idx].conn.broken_pipe {
                let client = self.clients.remove(idx);
                println!(
                    "Client {:?} connection interupted, disconnecting...",
                    client.name
                );
            }
        }

        // --- Use any generated chunks from the chunk builders ---
        while let Ok((pos, chunk, built_features)) = self.chunk_builder_recv.try_recv() {
            self.world.chunks.insert(pos, chunk);
            self.dirty_chunks.insert(pos);
            self.world.unplaced_features.extend(built_features);
        }

        // --- Send any updated chunks to all the clients ---
        for chunk_pos in &self.dirty_chunks {
            let chunk = self.world.get_chunk(*chunk_pos).unwrap();
            let nodes = Vec::from(chunk.used_nodes());

            for client in &mut self.clients {
                // If the chunk is not within render distance of the client,
                // don't bother sending it.
                if client.conn.broken_pipe || !client.using_chunk(*chunk_pos) {
                    continue;
                }
                if let Err(err) = client.conn.write(ClientCmd::GiveChunkData(
                    *chunk_pos,
                    nodes.clone(),
                    NodeAlloc::new(0..1, 1..2),
                )) {
                    println!(
                        "Error sending chunk data to client {:?} : {:?}",
                        client.name, err
                    );
                }
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

        let chunks_per_buider = 10;
        while self.chunk_builders.len() < 16 {
            let next: Vec<_> = (&mut chunks_to_build).take(chunks_per_buider).collect();
            if next.is_empty() {
                break;
            }
            for chunk in &next {
                _ = self.chunks_to_build_set.remove(chunk);
            }
            self.chunk_builders.push(ChunkBuilder::spawn(
                Arc::clone(&self.world.gen),
                next,
                self.chunk_builder_send.clone(),
            ));
        }
        self.chunks_to_build = chunks_to_build.collect();
    }

    pub fn update_world(&mut self) {
        for chunk_pos in self.world.place_features() {
            self.dirty_chunks.insert(chunk_pos);
        }
    }

    fn handle_client_cmd(&mut self, client: usize, cmd: ServerCmd, player_list: &[PlayerInfo]) {
        let client = &mut self.clients[client];

        match cmd {
            ServerCmd::Handshake { .. } => {}
            ServerCmd::DisconnectNotice => {
                client.conn.broken_pipe = true;
                println!("client sent disconnect notice {:?}", client.name)
            }
            ServerCmd::GetPlayersList => {
                if let Err(err) = client
                    .conn
                    .write(ClientCmd::PlayersList(player_list.to_vec()))
                {
                    println!("Error sending cmd to client {:?} : {:?}", client.name, err);
                }
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
                        if let Err(err) = client.conn.write(ClientCmd::GiveChunkData(
                            chunk_pos,
                            data.used_nodes().to_vec(),
                            NodeAlloc::new(0..1, 1..2),
                        )) {
                            println!("Error sending cmd to client {:?} : {:?}", client.name, err);
                        }
                    } else {
                        if !self.chunks_to_build_set.contains(&chunk_pos) {
                            self.chunks_to_build.push(chunk_pos);
                            self.chunks_to_build_set.insert(chunk_pos);
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
            ServerCmd::PlaceVoxelData(_v, _pos) => {}
        }
    }

    pub fn poll_clients(clients: &mut [Client]) -> PollResults {
        let mut remove_clients = vec![];
        let mut commands = vec![];

        for client_idx in 0..clients.len() {
            let client = &mut clients[client_idx];

            let rs = client.conn.try_read();

            let cmd = match rs {
                Ok(Some(cmd)) => cmd,
                Ok(None) => continue,
                Err(err) => {
                    println!(
                        "Failed to poll commands from client {:?} : {:?}",
                        client.name, err
                    );
                    remove_clients.push(client_idx);
                    continue;
                }
            };
            commands.push((client_idx, cmd));
        }
        PollResults {
            remove_clients,
            commands,
        }
    }

    pub fn handle_clients(&mut self) {
        let player_list = self.player_list();
        let poll = Self::poll_clients(&mut self.clients);
        for (client_idx, cmd) in poll.commands {
            println!(
                "handling cmd from client {:?} : {cmd:?}",
                &self.clients[client_idx].name
            );
            self.handle_client_cmd(client_idx, cmd, &player_list);
        }

        for client_idx in poll.remove_clients.into_iter().rev() {
            _ = self.clients.remove(client_idx);
        }
    }
}

pub struct PollResults {
    pub remove_clients: Vec<usize>,
    pub commands: Vec<(usize, ServerCmd)>,
}
