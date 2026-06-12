use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::cmd::{AccessLevel, Command, CommandDispatch};
use crate::connection::{Connection, TelnetConnection};
use crate::telnet::{codec::TelnetReader, INITIAL_NEGOTIATION};
use mud_core::World;

pub struct Server {
    bind_addr: String,
    world: Arc<Mutex<World>>,
    commands: CommandDispatch,
    next_conn_id: AtomicU64,
}

impl Server {
    pub fn new(bind_addr: impl Into<String>, world: World) -> Self {
        Server {
            bind_addr: bind_addr.into(),
            world: Arc::new(Mutex::new(world)),
            commands: CommandDispatch::new(),
            next_conn_id: AtomicU64::new(1),
        }
    }

    pub fn register_command(
        &mut self,
        name: &'static str,
        aliases: &'static [&'static str],
        access: AccessLevel,
        handler: crate::cmd::CommandFn,
    ) {
        self.commands.register(Command {
            name,
            aliases,
            access,
            handler,
        });
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        tracing::info!("Server listening on {}", self.bind_addr);

        let world = self.world;
        let commands = Arc::new(self.commands);

        loop {
            let (stream, addr) = listener.accept().await?;
            tracing::info!("New connection from {addr}");

            let conn_id = self.next_conn_id.fetch_add(1, Ordering::SeqCst);
            let world = world.clone();
            let commands = commands.clone();

            tokio::spawn(async move {
                handle_connection(conn_id, stream, world, commands).await;
            });
        }
    }
}

async fn handle_connection(
    conn_id: u64,
    stream: tokio::net::TcpStream,
    world: Arc<Mutex<World>>,
    commands: Arc<CommandDispatch>,
) {
    let (reader_half, mut writer_half) = stream.into_split();

    let (mut conn, mut output_rx) = TelnetConnection::new(conn_id);

    let output_handle = tokio::spawn(async move {
        if let Err(e) = writer_half.write_all(&INITIAL_NEGOTIATION).await {
            tracing::debug!("Connection {conn_id} write error: {e}");
            return;
        }
        while let Some(bytes) = output_rx.recv().await {
            if let Err(e) = writer_half.write_all(&bytes).await {
                tracing::debug!("Connection {conn_id} write error: {e}");
                break;
            }
        }
    });

    conn.send_line("Welcome to Mud!");
    conn.send_line("Type 'help' for commands.");
    conn.send_line("");

    let telnet_reader = TelnetReader::new(reader_half);
    let mut buf_reader = BufReader::new(telnet_reader);
    let mut line = String::new();

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => {
                tracing::info!("Connection {conn_id} closed");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                tracing::debug!("Connection {conn_id}: {trimmed}");

                let mut world_lock = world.lock().await;
                commands.execute(&mut world_lock, &mut conn, trimmed);
                drop(world_lock);
            }
            Err(e) => {
                tracing::debug!("Connection {conn_id} read error: {e}");
                break;
            }
        }
    }

    let _ = output_handle.await;
}
