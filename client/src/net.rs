use anyhow::Context;
use common::net::{ClientCmd, ConnError, ServerCmd};
use std::io::Read;
use std::net::{SocketAddr, TcpStream};

pub struct ServerConn {
    pub stream: TcpStream,
    pub received_bytes: Vec<u8>,
}
impl ServerConn {
    pub fn establish(addr: SocketAddr, name: String) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).context("Failed to establish TCP connection")?;
        let mut stream = Self {
            stream,
            received_bytes: vec![],
        };

        stream.write(ServerCmd::Handshake { name })?;
        let response = stream.read();
        match response? {
            ClientCmd::HandshakeAccepted => (),
            ClientCmd::HandshakeDenied => Err(ConnError::ServerDeniedConnection)?,
            _ => Err(ConnError::ServerGaveInvalidData)?,
        };
        Ok(stream)
    }

    pub fn write(&mut self, cmd: ServerCmd) -> anyhow::Result<()> {
        bincode::serde::encode_into_std_write(cmd, &mut self.stream, bincode::config::standard())
            .context("Failed to send message to server")?;
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
    pub fn read(&mut self) -> anyhow::Result<ClientCmd> {
        let cmd =
            bincode::serde::decode_from_std_read(&mut self.stream, bincode::config::standard())
                .context("Failed to read message from server")?;
        Ok(cmd)
    }
}
