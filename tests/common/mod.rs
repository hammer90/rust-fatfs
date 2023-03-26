use std::fs;

use fatfs::{DefaultTimeProvider, FsOptions, LossyOemCpConverter, StdIoWrapper};
use fscommon::BufStream;

pub const FAT12_IMG: &str = "fat12.img";
pub const FAT16_IMG: &str = "fat16.img";
pub const FAT32_IMG: &str = "fat32.img";
pub const IMG_DIR: &str = "resources";
pub const TMP_DIR: &str = "tmp";

pub type FileSystem = fatfs::FileSystem<StdIoWrapper<BufStream<fs::File>>, DefaultTimeProvider, LossyOemCpConverter>;

pub fn call_with_tmp_img<F: Fn(&str) -> ()>(f: F, filename: &str, test_seq: u32) {
    let _ = env_logger::builder().is_test(true).try_init();
    let img_path = format!("{}/{}", IMG_DIR, filename);
    let tmp_path = format!("{}/{}-{}", TMP_DIR, test_seq, filename);
    fs::create_dir(TMP_DIR).ok();
    fs::copy(&img_path, &tmp_path).unwrap();
    f(tmp_path.as_str());
    fs::remove_file(tmp_path).unwrap();
}

pub fn open_filesystem_rw(tmp_path: &str) -> FileSystem {
    let file = fs::OpenOptions::new().read(true).write(true).open(&tmp_path).unwrap();
    let buf_file = BufStream::new(file);
    let options = FsOptions::new().update_accessed_date(true);
    FileSystem::new(buf_file, options).unwrap()
}

pub fn call_with_fs<F: Fn(FileSystem) -> ()>(f: F, filename: &str, test_seq: u32) {
    let callback = |tmp_path: &str| {
        let fs = open_filesystem_rw(tmp_path);
        f(fs);
    };
    call_with_tmp_img(&callback, filename, test_seq);
}
