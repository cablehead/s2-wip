use scru128::Scru128Id;
use serde::{Deserialize, Serialize};
use ssri::Integrity;

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum MimeType {
    #[serde(rename = "text/plain")]
    TextPlain,
    #[serde(rename = "image/png")]
    ImagePng,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Content {
    pub hash: Option<Integrity>,
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
    pub hash: Integrity,
    pub stack_id: Option<Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct UpdatePacket {
    pub id: Scru128Id,
    pub source_id: Scru128Id,
    pub hash: Option<Integrity>,
    pub stack_id: Option<Scru128Id>,
    pub source: Option<String>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct ForkPacket {
    pub id: Scru128Id,
    pub source_id: Scru128Id,
    pub hash: Option<Integrity>,
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
        Store {
            packets,
            content,
            cache_path,
        }
    }

    pub fn cas_write(&self, content: &[u8], mime_type: MimeType) -> Integrity {
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

    pub fn cas_read(&self, hash: &Integrity) -> Option<Vec<u8>> {
        cacache::read_hash_sync(&self.cache_path, hash).ok()
    }

    pub fn insert_packet(&mut self, packet: &Packet) {
        let encoded: Vec<u8> = bincode::serialize(&packet).unwrap();
        self.packets
            .insert(packet.id().to_bytes(), encoded)
            .unwrap();
    }

    pub fn scan(&self) -> impl Iterator<Item = Packet> {
        self.packets.iter().filter_map(|item| {
            item.ok()
                .and_then(|(_, value)| bincode::deserialize::<Packet>(&value).ok())
        })
    }

    pub fn add(
        &mut self,
        content: &[u8],
        mime_type: MimeType,
        stack_id: Option<Scru128Id>,
        source: Option<String>,
    ) -> Packet {
        let hash = self.cas_write(content, mime_type);
        let packet = Packet::Add(AddPacket {
            id: scru128::new(),
            hash,
            stack_id,
            source,
        });
        self.insert_packet(&packet);
        packet
    }

    pub fn update(
        &mut self,
        source_id: Scru128Id,
        content: Option<&[u8]>,
        mime_type: MimeType,
        stack_id: Option<Scru128Id>,
        source: Option<String>,
    ) -> Packet {
        let hash = content.map(|c| self.cas_write(c, mime_type.clone()));
        let packet = Packet::Update(UpdatePacket {
            id: scru128::new(),
            source_id,
            hash,
            stack_id,
            source,
        });
        self.insert_packet(&packet);
        packet
    }

    pub fn fork(
        &mut self,
        source_id: Scru128Id,
        content: Option<&[u8]>,
        mime_type: MimeType,
        stack_id: Option<Scru128Id>,
        source: Option<String>,
    ) -> Packet {
        let hash = content.map(|c| self.cas_write(c, mime_type.clone()));
        let packet = Packet::Fork(ForkPacket {
            id: scru128::new(),
            source_id,
            hash,
            stack_id,
            source,
        });
        self.insert_packet(&packet);
        packet
    }

    pub fn delete(&mut self, source_id: Scru128Id) -> Packet {
        let packet = Packet::Delete(DeletePacket {
            id: scru128::new(),
            source_id,
        });
        self.insert_packet(&packet);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);

        let content = b"Hello, world!";
        let packet = store.add(content, MimeType::TextPlain, None, None);

        let stored_packet = store.scan().next().unwrap();
        assert_eq!(packet, stored_packet);

        match packet {
            Packet::Add(packet) => {
                let stored_content = store.cas_read(&packet.hash).unwrap();
                assert_eq!(content.to_vec(), stored_content);
            }
            _ => panic!("Expected AddPacket"),
        }
    }

    #[test]
    fn test_update() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);

        let content = b"Hello, world!";
        let packet = store.add(content, MimeType::TextPlain, None, None);

        let updated_content = b"Hello, updated world!";
        let update_packet = store.update(
            packet.id().clone(),
            Some(updated_content),
            MimeType::TextPlain,
            None,
            None,
        );

        let stored_update_packet = store.scan().last().unwrap();
        assert_eq!(update_packet, stored_update_packet);

        match update_packet {
            Packet::Update(packet) => {
                let stored_content = store.cas_read(&packet.hash.unwrap()).unwrap();
                assert_eq!(updated_content.to_vec(), stored_content);
            }
            _ => panic!("Expected UpdatePacket"),
        }
    }

    #[test]
    fn test_fork() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path);

        let content = b"Hello, world!";
        let packet = store.add(content, MimeType::TextPlain, None, None);

        let forked_content = b"Hello, forked world!";
        let forked_packet = store.fork(
            packet.id().clone(),
            Some(forked_content),
            MimeType::TextPlain,
            None,
            None,
        );

        let stored_fork_packet = store.scan().last().unwrap();
        assert_eq!(forked_packet, stored_fork_packet);

        match forked_packet {
            Packet::Fork(packet) => {
                let stored_content = store.cas_read(&packet.hash.unwrap()).unwrap();
                assert_eq!(forked_content.to_vec(), stored_content);
            }
            _ => panic!("Expected ForkPacket"),
        }
    }

    #[test]
    fn test_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let mut store = Store::new(path);
        let content = b"Hello, world!";
        let packet = store.add(content, MimeType::TextPlain, None, None);
        let delete_packet = store.delete(packet.id().clone());
        let stored_delete_packet = store.scan().last().unwrap();
        assert_eq!(delete_packet, stored_delete_packet);
    }
}
