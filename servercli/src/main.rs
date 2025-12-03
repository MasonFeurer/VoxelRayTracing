/*
A native application that uses blockworld-server to create a server and provides an interface through the cmdline.
*/

use anyhow::Context;
use server::{world::ServerWorld, Resources, ServerState};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let usage = "servercli (resource_folder) (port)";
    let mut args = std::env::args();
    _ = args.next(); // First arg is always the path to this program.

    let res_folder = args.next().expect(&format!(
        "Missing cmdline arg \"resource_folder\"\nUsage: {usage}"
    ));
    let port = args
        .next()
        .with_context(|| format!("Missing cmdline arg \"port\"\nUsage: {usage}"))?;
    let port: u16 = port
        .parse()
        .with_context(|| format!("Invalid cmdline arg \"port\"\nUsage: {usage}"))?;

    let address = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
    let resources = Resources::load(&res_folder).context("Failed to load resources")?;

    println!("Using address {address:?}...");

    let world = ServerWorld::new(
        &resources.world_presets[0],
        resources.world_features,
        fastrand::i64(..),
    );
    let mut server = ServerState::new(address, format!("My Dev Server"), world);

    server.start().context("Failed to start server")?;

    println!("Server is running.");
    let cli_cmds = spawn_cli(Arc::clone(&server.kill));
    loop {
        server.handle_clients();
        server.update();
        server.update_world();

        match cli_cmds.try_recv() {
            Ok(CliCmd::GetPlayers) => {
                if server.clients.len() == 0 {
                    println!("No players online!");
                }
                for client in &server.clients {
                    println!(
                        "- {:?} | ({:.2}, {:.2}, {:.2}) | {:?}",
                        client.name,
                        client.pos.x,
                        client.pos.y,
                        client.pos.z,
                        client.address()
                    );
                }
            }
            Ok(CliCmd::ShowWorldSummary) => {
                println!("--- World ---");
                println!("chunk count: {}", server.world.chunks.len());
                let mut lowest_chunk_space = u32::MAX;
                let mut used_space = 0;
                let mut allocated_space = 0;
                for (_pos, chunk) in &server.world.chunks {
                    let space = chunk.node_alloc.range.end;
                    allocated_space += space;
                    used_space += chunk.node_alloc.total_used_mem();
                    if space < lowest_chunk_space {
                        lowest_chunk_space = space;
                    }
                }
                println!("allocated space: {allocated_space}");
                println!(
                    "used space: {used_space} (%{})",
                    (used_space as f32 / allocated_space as f32) * 100.0
                );
                println!("least allocated by chunk: {lowest_chunk_space}");
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

pub fn spawn_cli(shutdown: Arc<AtomicBool>) -> Receiver<CliCmd> {
    let (send, recv) = channel();

    std::thread::spawn(move || {
        loop {
            let mut cmd_buf = String::new();
            _ = std::io::stdin().read_line(&mut cmd_buf);
            _ = cmd_buf.pop(); // remove the new-line character
            match cmd_buf.as_str() {
                "stop" => {
                    shutdown.store(true, Ordering::Relaxed);
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
