use anyhow::Context;
use common::net::{ClientCmd, ConnError, ServerCmd};
use glam::Vec3;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

pub struct ServerConn {
    pub stream: TcpStream,
    pub received_bytes: Vec<u8>,
    pub player_pos: Vec3,
}
impl ServerConn {
    pub fn establish(addr: SocketAddr, name: impl Into<String>) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).context("Failed to establish TCP connection")?;
        let mut stream = Self {
            stream,
            received_bytes: vec![],
            player_pos: Vec3::ZERO,
        };

        stream.write(ServerCmd::Handshake { name: name.into() })?;
        let response = stream.read();
        match response? {
            ClientCmd::HandshakeAccepted(player_pos) => stream.player_pos = player_pos,
            ClientCmd::HandshakeDenied => Err(ConnError::ServerDeniedConnection)?,
            _ => Err(ConnError::ServerGaveInvalidData)?,
        };
        Ok(stream)
    }

    pub fn write(&mut self, cmd: ServerCmd) -> anyhow::Result<()> {
        let bytes = bincode::serde::encode_to_vec(&cmd, bincode::config::standard())?;
        self.stream.write_all(&bytes).context("Failed to send message to server")?;
        Ok(())
    }

    /// Not blocking
    pub fn try_read(&mut self) -> anyhow::Result<Option<ClientCmd>> {
        self.stream.set_nonblocking(true)?;
        match self.stream.read_to_end(&mut self.received_bytes) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => Err(err)?,
        }
        self.stream.set_nonblocking(false)?;
        match bincode::serde::decode_from_slice(&self.received_bytes, bincode::config::standard()) {
            Ok((cmd, remainder)) => {
                self.received_bytes = self.received_bytes[remainder..].to_vec();
                Ok(Some(cmd))
            }
            Err(bincode::error::DecodeError::UnexpectedEnd { .. }) => Ok(None),
            Err(err) => Err(err)?,
        }
    }

    /// Blocking
    pub fn read(&mut self) -> anyhow::Result<ClientCmd> {
        loop {
            if let Some(cmd) = self.try_read()? {
                return Ok(cmd)
            }
            // std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}
