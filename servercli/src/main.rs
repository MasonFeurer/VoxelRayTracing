/*
A native application that uses blockworld-server to create a server and provides an interface through the cmdline.
*/

use server::ServerState;
use std::sync::mpsc::{channel, Receiver};
use std::{
    net::{SocketAddr, TcpStream},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    let address = std::env::args().nth(1).unwrap_or("127.0.0.1:60000".into());

    println!("Using address {address:?}");

    let mut server = ServerState::new(
        SocketAddr::from_str(&address).unwrap(),
        format!("My Dev Server"),
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
                let list_str = names.join(", ");
                println!("{list_str}");
            }
            Ok(CliCmd::Stop) => break,
            Err(_) => {}
        }

        std::thread::sleep(Duration::from_millis(100));
    }
    println!("SERVER CLI PROGRAM IS DONE");
    Ok(())
}

pub enum CliCmd {
    GetPlayers,
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
                    send.send(CliCmd::Stop);
                    break;
                }
                "players" => _ = send.send(CliCmd::GetPlayers),
                _ => println!("Error: Unrecognized command!"),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    recv
}
