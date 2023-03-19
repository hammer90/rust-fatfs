use std::fs::File;
use std::io;

use fatfs::{Cluster, FileSystem, FsOptions};
use fscommon::BufStream;

fn print_free(first_free: usize, first_used: usize) {
    println!("{} FREE", first_free);
    if first_free != first_used - 1 {
        println!("...");
        println!("{} FREE", first_used - 1);
    }
}

fn main() -> io::Result<()> {
    let file = File::open("resources/fat32.img")?;
    let buf_rdr = BufStream::new(file);
    let fs = FileSystem::new(buf_rdr, FsOptions::new())?;
    let map = fs.cluster_map()?;
    let mut free_start = None;
    for (pos, cluster) in map.iter().enumerate() {
        match cluster {
            Cluster::File(file_name) => {
                if let Some(prev_free) = free_start {
                    print_free(prev_free, pos);
                    free_start = None;
                }
                println!("{} File({})", pos, file_name);
            }
            Cluster::Directory(dir_name) => {
                if let Some(prev_free) = free_start {
                    print_free(prev_free, pos);
                    free_start = None;
                }
                println!("{} Dir({})", pos, dir_name);
            }
            Cluster::Free => {
                if free_start.is_none() {
                    free_start = Some(pos)
                }
            }
            Cluster::Fat => {
                println!("{} FAT", pos);
            }
        }
    }
    if let Some(prev_free) = free_start {
        print_free(prev_free, map.len());
    }
    Ok(())
}
