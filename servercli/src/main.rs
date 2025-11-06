/*
A native application that uses blockworld-server to create a server and provides an interface through the cmdline.
*/

use server::{Resources, ServerState};
use std::sync::mpsc::{channel, Receiver};
use std::{
    net::SocketAddr,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    let address = String::from("127.0.0.1:60000");
    let resfolder = std::env::args().nth(1).unwrap();

    let resources = Resources::load(&resfolder)?;

    println!("Using address {address:?}");

    let mut server = ServerState::new(
        SocketAddr::from_str(&address).unwrap(),
        format!("My Dev Server"),
        resources,
    );
    if let Err(err) = server.listen_for_clients(&SHUTDOWN_FLAG) {
        println!("Failed to listen for clients: {err:?}");
    }

    println!("Server is running.");
    let cli_cmds = spawn_cli();
    loop {
        server.process_clients();
        server.respond_to_clients();

        match cli_cmds.try_recv() {
            Ok(CliCmd::GetPlayers) => {
                let names: Vec<&str> = server
                    .clients
                    .iter()
                    .map(|client| client.name.as_str())
                    .collect();
                if names.len() == 0 {
                    println!("no players online!");
                } else {
                    println!("players: {}", names.join(", "));
                }
            }
            Ok(CliCmd::ShowWorldSummary) => {
                println!("--- World ---");
                println!("chunk count: {}", server.world.chunks.len());
                println!("chunks: {:#?}", server.world.chunks);
            }
            Ok(CliCmd::Stop) => break,
            Err(_) => {}
        }

        std::thread::sleep(Duration::from_millis(1));
    }
    println!("SERVER CLI PROGRAM IS DONE");
    Ok(())
}

pub enum CliCmd {
    GetPlayers,
    ShowWorldSummary,
    Stop,
}

pub fn spawn_cli() -> Receiver<CliCmd> {
    let (send, recv) = channel();

    std::thread::spawn(move || {
        loop {
            let mut cmd_buf = String::new();
            _ = std::io::stdin().read_line(&mut cmd_buf);
            _ = cmd_buf.pop(); // remove the new-line character
            match cmd_buf.as_str() {
                "stop" => {
                    SHUTDOWN_FLAG.store(true, Ordering::Relaxed);
                    _ = send.send(CliCmd::Stop);
                    break;
                }
                "players" => _ = send.send(CliCmd::GetPlayers),
                "world" => _ = send.send(CliCmd::ShowWorldSummary),
                _ => println!("Error: Unrecognized command!"),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    recv
}
