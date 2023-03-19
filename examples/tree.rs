use std::fs::File;
use std::io;

use fatfs::{Dir, Error, FileSystem, FsOptions, IoBase, OemCpConverter, ReadWriteSeek, TimeProvider};
use fscommon::BufStream;

fn iter_dir<IO, TP, OCC>(dir: Dir<IO, TP, OCC>, depth: u32) -> Result<(), Error<<IO as IoBase>::Error>>
where
    IO: ReadWriteSeek,
    TP: TimeProvider,
    OCC: OemCpConverter,
{
    for r in dir.iter() {
        let e = r?;
        let pad = (0..depth).map(|_| "│   ").collect::<String>();
        println!("{}├── {}", pad, e.file_name());
        if e.is_dir() && e.file_name() != "." && e.file_name() != ".." {
            iter_dir(e.to_dir(), depth + 1)?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let file = File::open("resources/fat32.img")?;
    let buf_rdr = BufStream::new(file);
    let fs = FileSystem::new(buf_rdr, FsOptions::new())?;
    let root_dir = fs.root_dir();
    iter_dir(root_dir, 0)?;
    Ok(())
}
