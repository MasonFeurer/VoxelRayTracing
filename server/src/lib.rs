/*
A library that provides an interface for creating and managing a BlockWorld server-world.
*/

pub mod net;
pub mod world;

use common::net::{ClientCmd, ServerCmd};
use common::server::PlayerInfo;
use glam::Vec3;
use net::ClientConn;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use world::Chunk;

pub struct ServerState {
    pub address: SocketAddr,
    pub name: String,
    pub clients: Vec<Client>,
    pub live_chunks: Vec<Chunk>,
    pub new_clients: Option<Receiver<Result<Client, anyhow::Error>>>,
}
impl ServerState {
    pub fn new(addr: SocketAddr, name: String) -> Self {
        Self {
            address: addr,
            name,
            clients: vec![],
            live_chunks: vec![],
            new_clients: None,
        }
    }

    pub fn process_clients(&mut self) {
        let Some(clients) = &self.new_clients else {
            return;
        };
        while let Ok(client) = clients.try_recv() {
            match client {
                Ok(client) => self.clients.push(client),
                Err(err) => {}
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
                ServerCmd::Handshake { name: String } => {}

                ServerCmd::DisconnectNotice => clients_disconnecting.push(idx),
                ServerCmd::GetPlayersList => {
                    if let Err(err) = client.conn.write(ClientCmd::PlayersList(list.clone())) {
                        println!(
                            "Error sending client list to client {:?} : {:?}",
                            client.name, err
                        );
                    }
                }
                ServerCmd::GetVoxelData(id, pos) => {}
                ServerCmd::GetChunkData(id, pos) => {}
                ServerCmd::PlaceVoxelData(v, pos) => {}
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
                                sender.send(Ok(client));
                            }
                            Err(err) => {
                                println!("Failed to connect client: {err:?}");
                                sender.send(Err(err));
                            }
                        };
                    }
                    Err(err) => {
                        println!("Failed to connect client : {err:?}");
                        sender.send(Err(err.into()));
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
