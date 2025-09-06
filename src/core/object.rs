use crate::core::Repository;
use std::fs;
use flate2::{Compression, write::ZlibEncoder, read::ZlibDecoder};
use std::io::{Write, Read};
use ring::digest;

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl ObjectType {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "blob" => Some(ObjectType::Blob),
            "tree" => Some(ObjectType::Tree),
            "commit" => Some(ObjectType::Commit),
            "tag" => Some(ObjectType::Tag),
            _ => None,
        }
    }
}

pub struct Object;

impl Object {
    pub fn create(
        repo: &Repository, 
        obj_type: ObjectType, 
        content: &[u8]
    ) -> Result<String, Box<dyn std::error::Error>> {
        let header = format!("{} {}\0", obj_type.as_str(), content.len());
        let mut full_content = header.into_bytes();
        full_content.extend_from_slice(content);
        
        let hash = hash_content(&full_content);
        let (dir, file) = hash.split_at(2);
        
        let obj_dir = repo.objects_dir().join(dir);
        fs::create_dir_all(&obj_dir)?;
        
        let obj_path = obj_dir.join(file);
        if !obj_path.exists() {
            let compressed = compress_data(&full_content)?;
            fs::write(&obj_path, compressed)?;
            
            Self::set_object_permissions(&obj_path)?;
            Self::verify_object_integrity(&obj_path, &hash)?;
        }
        
        Ok(hash)
    }

    pub fn read(repo: &Repository, hash: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if hash.len() < 8 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid object hash format".into());
        }
        
        let (dir, file) = hash.split_at(2);
        let obj_path = repo.objects_dir().join(dir).join(file);
        
        if !obj_path.exists() {
            return Err(format!("Object {} not found", hash).into());
        }
        
        let compressed_data = fs::read(&obj_path)?;
        let decompressed = decompress_data(&compressed_data)?;
        
        Self::verify_decompressed_data(&decompressed, hash)?;
        
        if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
            Ok(decompressed[null_pos + 1..].to_vec())
        } else {
            Err("Invalid object format: no null terminator found".into())
        }
    }

    pub fn read_with_type(
        repo: &Repository, 
        hash: &str
    ) -> Result<(ObjectType, Vec<u8>), Box<dyn std::error::Error>> {
        let (dir, file) = hash.split_at(2);
        let obj_path = repo.objects_dir().join(dir).join(file);
        
        let compressed_data = fs::read(&obj_path)?;
        let decompressed = decompress_data(&compressed_data)?;
        
        Self::verify_decompressed_data(&decompressed, hash)?;
        
        if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
            let header = String::from_utf8_lossy(&decompressed[..null_pos]);
            let parts: Vec<&str> = header.splitn(2, ' ').collect();
            
            if parts.len() == 2 {
                let obj_type = ObjectType::from_str(parts[0])
                    .ok_or("Unknown object type")?;
                let expected_size: usize = parts[1].parse()
                    .map_err(|_| "Invalid size in object header")?;
                
                let content = decompressed[null_pos + 1..].to_vec();
                
                if content.len() != expected_size {
                    return Err("Object size mismatch".into());
                }
                
                Ok((obj_type, content))
            } else {
                Err("Invalid object header format".into())
            }
        } else {
            Err("Invalid object format: no null terminator found".into())
        }
    }

    pub fn exists(repo: &Repository, hash: &str) -> bool {
        if hash.len() < 8 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        
        let (dir, file) = hash.split_at(2);
        let obj_path = repo.objects_dir().join(dir).join(file);
        obj_path.exists()
    }

    pub fn list_objects(repo: &Repository) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut objects = Vec::new();
        let objects_dir = repo.objects_dir();
        
        if !objects_dir.exists() {
            return Ok(objects);
        }
        
        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let dir_name = entry.file_name();
                if let Some(prefix) = dir_name.to_str() {
                    if prefix.len() == 2 && prefix.chars().all(|c| c.is_ascii_hexdigit()) {
                        for obj_entry in fs::read_dir(entry.path())? {
                            let obj_entry = obj_entry?;
                            if let Some(suffix) = obj_entry.file_name().to_str() {
                                if suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                                    objects.push(format!("{}{}", prefix, suffix));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        objects.sort();
        Ok(objects)
    }

    fn set_object_permissions(obj_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(obj_path)?.permissions();
            perms.set_mode(0o644);
            fs::set_permissions(obj_path, perms)?;
        }
        Ok(())
    }

    fn verify_object_integrity(
        obj_path: &std::path::Path, 
        expected_hash: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compressed_data = fs::read(obj_path)?;
        let decompressed = decompress_data(&compressed_data)?;
        let actual_hash = hash_content(&decompressed);
        
        if actual_hash != expected_hash {
            fs::remove_file(obj_path)?;
            return Err("Object integrity check failed".into());
        }
        
        Ok(())
    }

    fn verify_decompressed_data(
        data: &[u8], 
        expected_hash: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let actual_hash = hash_content(data);
        if actual_hash != expected_hash {
            return Err("Object integrity verification failed".into());
        }
        Ok(())
    }

    pub fn get_size(repo: &Repository, hash: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let (_, content) = Self::read_with_type(repo, hash)?;
        Ok(content.len() as u64)
    }

    pub fn verify_repository_objects(repo: &Repository) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut corrupted = Vec::new();
        let objects = Self::list_objects(repo)?;
        
        for hash in objects {
            match Self::read(repo, &hash) {
                Ok(_) => {},
                Err(_) => corrupted.push(hash),
            }
        }
        
        Ok(corrupted)
    }
}

pub fn hash_content(content: &[u8]) -> String {
    let digest_result = digest::digest(&digest::SHA256, content);
    hex::encode(digest_result.as_ref())
}

fn compress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn decompress_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
