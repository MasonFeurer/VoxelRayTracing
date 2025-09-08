// ONLY FOR DEVELOPMENT

use anyhow::{anyhow, Context};
use blockworld_client::GameState;
use common::net::{ClientCmd, ServerCmd};
use glam::IVec3;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum CliCmd {
    Stop,
    GetPlayers,
    GetChunk(IVec3),
}

fn spawn_cli() -> Receiver<CliCmd> {
    let (send, recv) = channel();

    std::thread::spawn(move || loop {
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        let mut cmd_str = String::new();
        _ = std::io::stdin().read_line(&mut cmd_str);
        _ = cmd_str.pop(); // remove the new-line character

        let cmd_parts: Vec<String> = if cmd_str.is_empty() {
            vec![]
        } else {
            cmd_str.split(" ").map(String::from).collect()
        };
        if cmd_parts.len() == 0 {
            continue;
        }

        match cmd_parts[0].as_str() {
            "stop" => {
                send.send(CliCmd::Stop);
            }
            "players" => {
                send.send(CliCmd::GetPlayers);
            }
            "getchunk" => {
                if cmd_parts.len() < 4 {
                    println!("Expected 3 integers to represent the chunk position");
                    continue;
                }
                let Ok(x): Result<i32, _> = cmd_parts[1].parse() else {
                    println!("Invalid integer: {:?}", cmd_parts[1]);
                    continue;
                };
                let Ok(y): Result<i32, _> = cmd_parts[2].parse() else {
                    println!("Invalid integer: {:?}", cmd_parts[2]);
                    continue;
                };
                let Ok(z): Result<i32, _> = cmd_parts[3].parse() else {
                    println!("Invalid integer: {:?}", cmd_parts[3]);
                    continue;
                };

                send.send(CliCmd::GetChunk(IVec3 { x, y, z }));
            }
            "getvoxel" => {
                todo!()
            }
            _ => println!("Error: Unrecognized command!"),
        }
    });
    recv
}

pub fn run_cmd(cmd: CliCmd, client: &mut GameState, stop: &mut bool) -> anyhow::Result<()> {
    match cmd {
        CliCmd::Stop => {
            client
                .disconnect()
                .context("Failed to disconnect to server")?;
            println!("Sent disconnect notice");
            *stop = true;
        }
        CliCmd::GetPlayers => {
            client
                .send_cmd(ServerCmd::GetPlayersList)
                .context("Failed to send GetPlayersList command")?;
            let rs = client.recv_cmd()?;
            let ClientCmd::PlayersList(list) = rs else {
                return Err(anyhow!("Unexpected command from server : {:?}", rs));
            };
            println!("players: {:?}", list);
        }
        CliCmd::GetChunk(pos) => {
            client
                .send_cmd(ServerCmd::GetChunkData(0, pos))
                .context("Failed to send GetChunkData command");
        }
    }
    Ok(())
}

pub fn main() -> anyhow::Result<()> {
    let args = std::env::args();
    let username = std::env::args()
        .nth(1)
        .expect("Missing 1st cmdline argument 'username'");
    let port = std::env::args()
        .nth(2)
        .expect("Missing 2nd cmdline argument 'port'");
    let port: u16 = port.parse().expect("Invalid port address");

    let mut client = GameState::new(username);
    client.join_server(SocketAddr::new("127.0.0.1".parse().unwrap(), port))?;

    println!("Connected to server!");

    let cli_cmds = spawn_cli();

    loop {
        let mut stop = false;
        if let Ok(cmd) = cli_cmds.try_recv() {
            if let Err(err) = run_cmd(cmd, &mut client, &mut stop) {
                println!("ERROR: {err:?}");
            }
        }
        if stop {
            break;
        }
    }
    Ok(())
}
