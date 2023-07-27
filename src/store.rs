use serde::{Deserialize, Serialize};

use scru128::Scru128Id;

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum MimeType {
    #[serde(rename = "text/plain")]
    TextPlain,
    #[serde(rename = "image/png")]
    ImagePng,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Content {
    pub hash: Option<ssri::Integrity>,
    pub mime_type: MimeType,
    pub terse: String,
    pub tiktokens: usize,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
    Add(AddPacket),
    Update(UpdatePacket),
    Fork(ForkPacket),
    Delete(DeletePacket),
}

impl Packet {
    pub fn id(&self) -> &Scru128Id {
        match self {
            Packet::Add(packet) => &packet.id,
            Packet::Update(packet) => &packet.id,
            Packet::Fork(packet) => &packet.id,
            Packet::Delete(packet) => &packet.id,
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct AddPacket {
    pub id: Scru128Id,
    pub hash: ssri::Integrity,
    pub stack_id: Option<Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct UpdatePacket {
    pub id: Scru128Id,
    pub source_id: Scru128Id,
    pub hash: Option<ssri::Integrity>,
    pub stack_id: Option<Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct ForkPacket {
    pub id: Scru128Id,
    pub source_id: Scru128Id,
    pub hash: Option<ssri::Integrity>,
    pub stack_id: Option<Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct DeletePacket {
    pub id: Scru128Id,
    pub source_id: Scru128Id,
}

pub struct Store {
    packets: sled::Tree,
    content: sled::Tree,
    pub cache_path: String,
}

impl Store {
    pub fn new(path: &str) -> Store {
        let db = sled::open(std::path::Path::new(path).join("index")).unwrap();
        let packets = db.open_tree("packets").unwrap();
        let content = db.open_tree("content").unwrap();
        let cache_path = std::path::Path::new(path)
            .join("cas")
            .into_os_string()
            .into_string()
            .unwrap();
        Store { packets, content, cache_path }
    }

    pub fn cas_write(&self, content: &[u8], mime_type: MimeType) -> ssri::Integrity {
        let hash = cacache::write_hash_sync(&self.cache_path, content).unwrap();
        let content = Content {
            hash: Some(hash.clone()),
            mime_type,
            terse: String::from_utf8_lossy(content).into_owned(),
            tiktokens: content.len(),
        };
        let encoded: Vec<u8> = bincode::serialize(&content).unwrap();
        self.content.insert(hash.to_string(), encoded).unwrap();
        hash
    }

    pub fn cas_read(&self, hash: &ssri::Integrity) -> Option<Vec<u8>> {
        cacache::read_hash_sync(&self.cache_path, hash).ok()
    }

    pub fn insert_packet(&mut self, packet: &Packet) {
        let encoded: Vec<u8> = bincode::serialize(&packet).unwrap();
        self.packets.insert(packet.id().to_bytes(), encoded).unwrap();
    }

    pub fn scan(&self) -> impl Iterator<Item = Packet> {
        self.packets.iter().filter_map(|item| {
            item.ok()
                .and_then(|(_, value)| bincode::deserialize::<Packet>(&value).ok())
        })
    }
}
