use std::fs::File;
use std::path::Path;
use std::io::{BufReader, Error, Read};
use sha2::{Digest, Sha256};
use md5::{Md5};

const BLOCK_SIZE: usize = 1024 * 1024;

pub fn sha256_digest(path: &Path) -> Result<String, Error> {
    let handle = File::open(path)?;
    let mut reader = BufReader::new(handle);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; BLOCK_SIZE];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(format!("{:X}", digest))
}

pub fn md5_digest(path: &Path) -> Result<String, Error> {
    let handle = File::open(path)?;
    let mut reader = BufReader::new(handle);

    let digest = {
        let mut hasher = Md5::new();
        let mut buffer = vec![0; BLOCK_SIZE];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(format!("{:X}", digest))
}

mod tests {
    use super::{sha256_digest, md5_digest};
    use std::path::Path;

    const TEST_FILE: &str = "m71-2014.xyz";
    #[test]
    fn test_sha256() {
        let path_to_test = Path::new(TEST_FILE);
        let digest = sha256_digest(&path_to_test).unwrap();
        println!("{}", digest);
    }

    #[test]
    fn test_md5() {
        let path_to_test = Path::new(TEST_FILE);
        let digest = md5_digest(&path_to_test).unwrap();
        println!("{}", digest);
    }
}
