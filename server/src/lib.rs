/*
A library that provides an interface for creating and managing a BlockWorld server-world.
*/

pub mod net;
pub mod world;

pub use common;

use common::net::{ClientCmd, ServerCmd};
use common::resources::{loader, VoxelPack, WorldFeatures, WorldPreset};
use common::server::PlayerInfo;
use glam::Vec3;
use net::ClientConn;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use world::ServerWorld;

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

pub struct ServerState {
    pub address: SocketAddr,
    pub name: String,
    pub clients: Vec<Client>,
    pub world: ServerWorld,
    pub new_clients: Option<Receiver<Result<Client, anyhow::Error>>>,
    pub resources: Resources,
}
impl ServerState {
    pub fn new(addr: SocketAddr, name: String, res: Resources) -> Self {
        Self {
            address: addr,
            name,
            clients: vec![],
            world: ServerWorld::new(),
            new_clients: None,
            resources: res,
        }
    }

    pub fn process_clients(&mut self) {
        let Some(clients) = &self.new_clients else {
            return;
        };
        while let Ok(client) = clients.try_recv() {
            match client {
                Ok(client) => self.clients.push(client),
                Err(_err) => {}
            }
        }
    }

    pub fn respond_to_clients(&mut self) {
        let mut clients_disconnecting = vec![];
        let list: Vec<_> = self
            .clients
            .iter()
            .map(|client| PlayerInfo {
                name: client.name.clone(),
                pos: client.pos,
            })
            .collect();
        for (idx, client) in self.clients.iter_mut().enumerate() {
            let rs = client.conn.try_read();
            if let Err(err) = rs {
                println!(
                    "Error polling commands from client {:?} : {:?}",
                    client.name, err
                );
                continue;
            }
            let Ok(Some(cmd)) = rs else {
                continue;
            };
            println!("Recieved cmd from client {:?} : {:?}", client.name, cmd);
            match cmd {
                ServerCmd::Handshake { .. } => {
                    println!("Unexpectedly received Handshake cmd from {:?}", client.name);
                }

                ServerCmd::DisconnectNotice => clients_disconnecting.push(idx),
                ServerCmd::GetPlayersList => {
                    if let Err(err) = client.conn.write(ClientCmd::PlayersList(list.clone())) {
                        println!(
                            "Error sending client list to client {:?} : {:?}",
                            client.name, err
                        );
                    }
                }
                ServerCmd::GetVoxelData(_id, _pos) => {}
                ServerCmd::GetChunkData(id, pos) => {
                    if self.world.get_chunk(pos).is_none() {
                        self.world.create_dev_chunk(pos, &self.resources);
                    }
                    let chunk = self.world.get_chunk(pos).unwrap();
                    let nodes = Vec::from(chunk.used_nodes());

                    if let Err(err) = client.conn.write(ClientCmd::GiveChunkData(id, pos, nodes)) {
                        println!(
                            "Error sending chunk data to client {:?} : {:?}",
                            client.name, err
                        );
                    }
                }
                ServerCmd::PlaceVoxelData(_v, _pos) => {}
            }
        }
        for idx in clients_disconnecting.iter().rev() {
            self.clients.remove(*idx);
        }
    }

    pub fn listen_for_clients(
        &mut self,
        shutdown: &'static AtomicBool,
    ) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(&self.address)?;

        let (sender, receiver) = channel();
        self.new_clients = Some(receiver);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if shutdown.load(Ordering::Relaxed) {
                    println!("Dropping TCPListener...");
                    break;
                }
                match stream {
                    Ok(stream) => {
                        println!("Client connected!");
                        match ClientConn::establish(stream) {
                            Ok((conn, name)) => {
                                let client = Client {
                                    name,
                                    pos: Vec3::ZERO,
                                    address: conn.stream.local_addr().unwrap(),
                                    conn,
                                };
                                _ = sender.send(Ok(client));
                            }
                            Err(err) => {
                                println!("Failed to connect client: {err:?}");
                                _ = sender.send(Err(err));
                            }
                        };
                    }
                    Err(err) => {
                        println!("Failed to connect client : {err:?}");
                        _ = sender.send(Err(err.into()));
                    }
                }
            }
            println!("DONE LISTENING FOR CLIENTS");
        });
        Ok(())
    }
}

pub struct Client {
    pub name: String,
    pub pos: Vec3,
    pub address: SocketAddr,
    pub conn: ClientConn,
}
