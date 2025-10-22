use anyhow::Context;
use common::net::{ClientCmd, ConnError, ServerCmd};
use std::io::Read;
use std::net::TcpStream;

pub struct ClientConn {
    pub stream: TcpStream,
    pub received_bytes: Vec<u8>,
}
impl ClientConn {
    pub fn establish(stream: TcpStream) -> anyhow::Result<(Self, String)> {
        let mut conn = Self {
            stream,
            received_bytes: vec![],
        };

        let cmd = conn.read()?;
        let name = if let ServerCmd::Handshake { name } = cmd {
            name
        } else {
            Err(ConnError::ClientGaveInvalidData)?
        };
        conn.write(ClientCmd::HandshakeAccepted)?;

        Ok((conn, name))
    }

    /// Not blocking
    pub fn try_read(&mut self) -> anyhow::Result<Option<ServerCmd>> {
        self.stream.set_nonblocking(true)?;
        match self.stream.read_to_end(&mut self.received_bytes) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => Err(err)?,
        }
        self.stream.set_nonblocking(false)?;
        let cmd =
            bincode::serde::decode_from_slice(&self.received_bytes, bincode::config::standard());
        match cmd {
            Ok((cmd, n)) => {
                self.received_bytes = self.received_bytes[n..].to_vec();
                Ok(Some(cmd))
            }
            Err(bincode::error::DecodeError::UnexpectedEnd { .. }) => Ok(None),
            Err(err) => Err(err)?,
        }
    }

    /// Blocking
    pub fn read(&mut self) -> anyhow::Result<ServerCmd> {
        if self.received_bytes.len() == 0 {
            let cmd =
                bincode::serde::decode_from_std_read(&mut self.stream, bincode::config::standard())
                    .context("Failed to read message from server")?;
            Ok(cmd)
        } else {
            todo!()
        }
    }

    pub fn write(&mut self, cmd: ClientCmd) -> anyhow::Result<()> {
        bincode::serde::encode_into_std_write(cmd, &mut self.stream, bincode::config::standard())
            .context("Failed to send message to client")?;
        Ok(())
    }
}
