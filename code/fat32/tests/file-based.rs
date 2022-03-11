use fat32::{Block, BlockDeviceError, BlockID, FileReader, Filesystem, BLOCK_SIZE};
use futures::{pin_mut, StreamExt};
use std::{io::SeekFrom, path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Mutex,
};

#[derive(Clone)]
struct FileBlockDevice {
    file: Arc<Mutex<File>>,
}

impl FileBlockDevice {
    async fn new(path: PathBuf) -> std::io::Result<Self> {
        let file = Arc::new(Mutex::new(File::open(path).await?));
        Ok(Self { file })
    }

    async fn read(&self, address: BlockID) -> Result<Block, BlockDeviceError<std::io::Error>> {
        let mut buf = [0u8; BLOCK_SIZE];
        let mut file = self.file.lock().await;
        let offset = address.offset();

        println!("Read block {:?} (offset={})", address, offset);

        file.seek(SeekFrom::Start(offset as u64))
            .await
            .map_err(BlockDeviceError::DeviceError)?;

        file.read_exact(&mut buf)
            .await
            .map_err(BlockDeviceError::DeviceError)?;

        Ok(Block::new(buf))
    }

    async fn write(
        &self,
        address: BlockID,
        block: Block,
    ) -> Result<(), BlockDeviceError<std::io::Error>> {
        unimplemented!()
    }
}

#[tokio::test]
async fn do_stuff() {
    let file = FileBlockDevice::new("/Users/tibl/Downloads/FAT32.dmg".into())
        .await
        .unwrap();
    let file_clone = file.clone();

    let filesystem = Filesystem::new(
        |address| file.read(address),
        |address, block| file_clone.write(address, block),
    )
    .await
    .unwrap();

    dbg!(&filesystem.mbr);
    dbg!(&filesystem.partition_index);
    dbg!(&filesystem.vid);

    let root_dir = filesystem.root_directory();
    let stream = filesystem.enumerate_directory(root_dir);
    pin_mut!(stream);

    while let Some(entry) = stream.next().await {
        match entry {
            Ok(entry) => {
                dbg!(entry);
            }
            Err(error) => {
                dbg!(error);
                break;
            }
        }
    }

    let file = filesystem.find_file("HELLO", "TXT").await.unwrap().unwrap();
    println!("FOUND FILE: {file:?}");

    let file_size = file.size();
    let mut reader = FileReader::new(file, &filesystem);

    let mut content = String::new();
    for i in 0..file_size {
        let byte = reader.read(i).await.unwrap();
        content.push(byte as char);
    }
    println!("File content: '{content}'");

    panic!();
}
