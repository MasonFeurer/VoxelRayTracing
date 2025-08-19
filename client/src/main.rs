// ONLY FOR DEVELOPMENT

use anyhow::Context;
use blockworld_client::GameClient;
use common::net::{ClientCmd, ServerCmd};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum CliCmd {
    Stop,
    GetPlayers,
}

fn spawn_cli() -> Receiver<CliCmd> {
    let (send, recv) = channel();

    std::thread::spawn(move || loop {
        let mut cmd_buf = String::new();
        _ = std::io::stdin().read_line(&mut cmd_buf);
        _ = cmd_buf.pop(); // remove the new-line character

        match cmd_buf.as_str() {
            "stop" => {
                send.send(CliCmd::Stop);
            }
            "getplayers" => {
                send.send(CliCmd::GetPlayers);
            }
            "getvoxel" => {
                todo!()
            }
            _ => println!("Error: Unrecognized command!"),
        }
    });
    recv
}

pub fn main() -> anyhow::Result<()> {
    let user_name = {
        println!("Enter a user-name: ");
        let mut cmd_buf = String::new();
        _ = std::io::stdin().read_line(&mut cmd_buf);
        _ = cmd_buf.pop(); // remove the new-line character
        cmd_buf
    };

    let mut client = GameClient::new(user_name);
    client.join_local_server()?;

    let cli_cmds = spawn_cli();

    loop {
        match cli_cmds.try_recv() {
            Ok(CliCmd::Stop) => {
                client
                    .disconnect()
                    .context("Failed to disconnect to server")?;
                println!("Send disconnect notice");
                break;
            }
            Ok(CliCmd::GetPlayers) => {
                client
                    .send_cmd(ServerCmd::GetPlayersList)
                    .context("Failed to send GetPlayersList to server")?;
                let rs = client.recv_cmd()?;
                let ClientCmd::PlayersList(list) = rs else {
                    println!("Unexpected command from server : {:?}", rs);
                    continue;
                };
                println!("players: {:?}", list);
            }
            Err(_) => {}
        }
    }
    Ok(())
}
