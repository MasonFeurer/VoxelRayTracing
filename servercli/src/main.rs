/*
A native application that uses blockworld-server to create a server and provides an interface through the cmdline.
*/

use anyhow::Context;
use server::{Resources, ServerState};
use std::sync::mpsc::{channel, Receiver};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    let usage = "servercli (resource_folder) (port)";
    let mut args = std::env::args();
    _ = args.next(); // First arg is always the path to this program.

    let res_folder = args.next().expect(&format!(
        "Missing cmdline arg \"resource_folder\"\nUsage: {usage}"
    ));
    let port = args
        .next()
        .expect(&format!("Missing cmdline arg \"port\"\nUsage: {usage}"));
    let port: u16 = port
        .parse()
        .with_context(|| "Invalid port address")
        .expect(&format!("Invalid cmdline arg \"port\"\nUsage: {usage}"));

    let address = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
    let resources = Resources::load(&res_folder)?;

    println!("Using address {address:?}");

    let mut server = ServerState::new(address, format!("My Dev Server"), resources);
    if let Err(err) = server.listen_for_clients(&SHUTDOWN_FLAG) {
        println!("Failed to listen for clients: {err:?}");
    }

    println!("Server is running.");
    let cli_cmds = spawn_cli();
    loop {
        server.process_clients();
        server.respond_to_clients();
        server.place_world_features();

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
