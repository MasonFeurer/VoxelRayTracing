/*
A native application that uses blockworld-server to create a server and provides an interface through the cmdline.
*/
use std::collections::{HashMap, HashSet};
use anyhow::Context;
use server::common::resources::Datapack;
use server::{world::ServerWorld, ServerState};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use bincode::{Decode, Encode};
use glam::{ivec3, IVec3, UVec3};
use server::common::log::{info, warn};
use server::common::resources::loader::RawWorldMeta;
use server::common::world::{Node, NodeAlloc, NodeRange};
use server::world::{ServerChunk, WorldFsExt};

const REGION_SIZE: u32 = 16; // how many chunks in a region

pub fn chunk_pos_to_region_pos(pos: IVec3) -> (IVec3, UVec3) {
    let origin = pos.div_euclid(IVec3::splat(REGION_SIZE as i32));
    let pos_in_region = (pos - (origin * REGION_SIZE as i32)).as_uvec3();
    (origin, pos_in_region)
}
pub fn pos_in_region_to_chunk_pos(region_pos: IVec3, pos_in_region: UVec3) -> IVec3 {
    region_pos * REGION_SIZE as i32 + pos_in_region.as_ivec3()
}
pub fn region_pos_to_chunk_pos(pos: IVec3) -> IVec3 {
    pos * REGION_SIZE as i32
}

pub fn region_path_by_pos(world_folder: impl AsRef<Path>, pos: IVec3) -> PathBuf {
    world_folder.as_ref().join(format!("regions/r_{}_{}_{}_.data", pos.x, pos.y, pos.z))
}

pub fn node_slice_from_bytes(bytes: &[u8]) -> &[Node] {
    assert_eq!(bytes.len() % size_of::<Node>(), 0, "Node slice size is not aligned to 4 bytes");
    assert_eq!(bytes.as_ptr() as usize % size_of::<Node>(), 0, "Node slice address is not aligned to 4 bytes");
    unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const Node, bytes.len() / size_of::<Node>()) }
}
pub fn node_slice_into_bytes(nodes: &[Node]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(nodes.as_ptr() as *const u8, nodes.len() * size_of::<Node>()) }
}

#[derive(Debug, Default, Decode, Encode)]
pub struct RegionFileHeader {
    chunks: HashMap<[u32; 3], NodeRange>,
}

#[derive(Debug, Default)]
pub struct RegionFile {
    header: RegionFileHeader,
    nodes: Vec<Node>,
}
impl RegionFile {
    pub fn append_chunk(&mut self, pos: UVec3, chunk: &[Node]) {
        let range = self.nodes.len() as u32..self.nodes.len() as u32 + chunk.len() as u32;
        self.header.chunks.insert(pos.to_array(), range);
        self.nodes.extend_from_slice(chunk);
    }
    pub fn read_chunk_data(&self, local_pos: UVec3) -> Option<&[Node]> {
        let chunk = self.header.chunks.get(&local_pos.to_array())?;
        Some(&self.nodes[chunk.start as usize..chunk.end as usize])
    }
    pub fn from_chunk(pos: UVec3, nodes: &[Node]) -> Self {
        let mut header = RegionFileHeader::default();
        header.chunks.insert(pos.to_array(), 0..nodes.len() as u32);
        Self { header, nodes: nodes.to_vec() }
    }
    pub fn from_file(bytes: &[u8]) -> Option<Self> {
        let (header, num_bytes): (RegionFileHeader, usize) = bincode::decode_from_slice(bytes, bincode::config::standard()).ok()?;
        let nodes = node_slice_from_bytes(&bytes[num_bytes..]).to_vec();
        Some(Self { header, nodes })
    }
    pub fn to_file(&self) -> Vec<u8> {
        let mut header = bincode::encode_to_vec(&self.header, bincode::config::standard()).unwrap();
        let nodes = node_slice_into_bytes(&self.nodes);
        header.extend_from_slice(&nodes);
        header
    }
}

#[derive(Default)]
pub struct ChunkCache {
    chunks: HashMap<IVec3, ServerChunk>,
}
impl ChunkCache {
    pub fn get(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }
    pub fn insert(&mut self, pos: IVec3, chunk: ServerChunk) {
        self.chunks.insert(pos, chunk);
    }
    pub fn remove(&mut self, pos: IVec3) {
        self.chunks.remove(&pos);
    }
}

pub struct WorldFs {
    world_folder: PathBuf,
    pub world_meta: RawWorldMeta,

    cache: RwLock<ChunkCache>,

    // chunk_cache: RwLock<ChunkCache>,
    available_chunks: HashSet<IVec3>,
    //                 region_pos,   pos_in_region
    dirty_chunks: HashMap<IVec3, HashSet<UVec3>>,
}
impl WorldFs {
    pub fn add_dirty_chunk(&mut self, chunk_pos: IVec3) {
        let (region_pos, pos_in_region) = chunk_pos_to_region_pos(chunk_pos);
        if let Some(region) = self.dirty_chunks.get_mut(&region_pos) {
            region.insert(pos_in_region);
        } else {
            self.dirty_chunks.insert(region_pos, [pos_in_region].into());
        }
    }
    pub fn save(&self, world: &ServerWorld) {
        let chunk_count = self.dirty_chunks.iter().map(|(_, chunks)| chunks.len()).sum::<usize>();

        info!("(WorldFs::save) saving dirty chunks : {:?} chunks", chunk_count);
        for (region_pos, dirty_chunks) in &self.dirty_chunks {
            let region_path = region_path_by_pos(&self.world_folder, *region_pos);

            let mut region = match std::fs::read(&region_path) {
                Ok(bytes) => RegionFile::from_file(&bytes).unwrap(),
                Err(_) => RegionFile::default(),
            };

            let mut new_region = RegionFile::default();
            for chunk_pos in dirty_chunks {
                let global_pos = pos_in_region_to_chunk_pos(*region_pos, *chunk_pos);
                let chunk = world.chunks.get(&global_pos).unwrap();

                new_region.append_chunk(*chunk_pos, &chunk.nodes);
                region.header.chunks.remove(&chunk_pos.to_array());
            }
            for (chunk_pos, chunk) in region.header.chunks {
                new_region.append_chunk(chunk_pos.into(), &region.nodes[chunk.start as usize..chunk.end as usize]);
            }
            std::fs::write(&region_path, new_region.to_file()).expect("Failed to write region file");
        }
    }

    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let world_folder = path.as_ref().to_path_buf();
        let world_meta: RawWorldMeta = ron::de::from_str(&std::fs::read_to_string(world_folder.join("meta.ron"))?)?;

        let region_dir = world_folder.join("regions");

        let mut available_chunks = HashSet::new();
        for file in std::fs::read_dir(region_dir)?.filter_map(|e| {
            match e {
                Err(e) => {
                    warn!("Failed to read region file: {}", e);
                    None
                }
                Ok(e) => Some(e)
            }
        }) {
            let name = file.file_name().to_string_lossy().to_string();
            let parts: Vec<&str>  = name.split("_").collect();

            if parts.len() != 5 { continue }
            // parts[0] = "r"
            let x = if let Ok(v) = parts[1].parse::<i32>() { v } else { continue };
            let y = if let Ok(v) = parts[2].parse::<i32>() { v } else { continue };
            let z = if let Ok(v) = parts[3].parse::<i32>() { v } else { continue };
            // parts[4] = ".data"

            let region_pos = IVec3::new(x, y, z);

            let mut file = std::fs::File::open(file.path())?;
            let header: RegionFileHeader = bincode::decode_from_std_read(&mut file, bincode::config::standard())?;
            for chunk_pos in header.chunks.keys() {
                let chunk_pos = pos_in_region_to_chunk_pos(region_pos, UVec3::from_array(*chunk_pos));
                available_chunks.insert(chunk_pos);
            }
        }
        Ok(Self {
            world_folder,
            world_meta,
            cache: RwLock::new(ChunkCache::default()),
            available_chunks,
            dirty_chunks: HashMap::new(),
        })
    }
}
impl WorldFsExt for WorldFs {
    fn read_chunk(&self, pos: IVec3) -> Option<ServerChunk> {
        let (region_pos, pos_in_region) = chunk_pos_to_region_pos(pos);

        if let Some(chunk) = self.cache.read().unwrap().get(pos) {
            return Some(chunk.clone())
        }

        _ = self.available_chunks.get(&pos)?;
        let region_path = region_path_by_pos(&self.world_folder, region_pos);
        let region_bytes = std::fs::read(&region_path).expect("Failed to read region file");
        let region = RegionFile::from_file(&region_bytes).expect("Failed to parse region file");
        let nodes = region.read_chunk_data(pos_in_region)?;

        let alloc = NodeAlloc::new(0..nodes.len() as u32, nodes.len() as u32..nodes.len() as u32 + 256);
        let chunk = ServerChunk { nodes: nodes.to_vec(), node_alloc: alloc};
        Some(chunk)
    }
}

fn main() -> anyhow::Result<()> {
    let usage = "servercli (datapack_folder) (world_folder) (port)";
    let mut args = std::env::args();
    _ = args.next(); // First arg is always the path to this program.

    let res_folder = args.next().expect(&format!(
        "Missing cmdline arg \"datapack_folder\"\nUsage: {usage}"
    ));
    let world_folder = args.next().expect(&format!(
        "Missing cmdline arg \"world_folder\"\nUsage: {usage}"
    ));
    let world_folder_path = PathBuf::from(&world_folder);

    let port = args
        .next()
        .with_context(|| format!("Missing cmdline arg \"port\"\nUsage: {usage}"))?;
    let port: u16 = port
        .parse()
        .with_context(|| format!("Invalid cmdline arg \"port\"\nUsage: {usage}"))?;

    let address = SocketAddr::new("127.0.0.1".parse()?, port);

    info!("Opening world {world_folder:?}...\n");

    let world_meta: RawWorldMeta = ron::de::from_str(&std::fs::read_to_string(world_folder_path.join("meta.ron"))?)?;
    let mut world_fs = WorldFs::open(world_folder_path)?;


    info!("Loading resources from {res_folder:?}...\n");

    let datapack = Datapack::load_from(&res_folder).context("Failed to load resources")?;

    let world = ServerWorld::new(
        &datapack.world_presets[0],
        datapack.world_features,
        world_meta.seed,
    );

    info!("Using address {address:?}...\n");
    let mut server = ServerState::new(address, "My Dev Server".to_string(), world);

    server.start().context("Failed to start server")?;

    info!("Server is running.");
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
            Ok(CliCmd::LoadChunk) => {
                server.chunks_to_build.push(ivec3(0, 0, 0));
                println!("Chunk 0,0,0 is now being built.");
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

    for chunk in server.world.chunks.keys() {
        world_fs.add_dirty_chunk(*chunk);
    }

    info!("Server has been stopped. Saving chunks to disk...");
    world_fs.save(&server.world);
    Ok(())
}

pub enum CliCmd {
    GetPlayers,
    ShowWorldSummary,
    Stop,
    LoadChunk,
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
                "loadchunk" => _ = send.send(CliCmd::LoadChunk),
                _ => println!("Error: Unrecognized command!"),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    recv
}
