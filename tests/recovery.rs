use std::convert::TryInto;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::str;

use fatfs::{ClusterBelongs, Recovery, RecoveryFactory, Write};

mod common;

use common::*;

fn copy_bytes(source: &[u8], target: &mut [u8], offset: usize) {
    let slice = &mut target[offset..offset + source.len()];
    slice.clone_from_slice(&source);
}

fn read_u32(source: &[u8], offset: usize) -> Option<u32> {
    if source.len() < offset + 4 {
        return None;
    }
    let slice = &source[offset..offset + 4];
    Some(u32::from_le_bytes(slice.try_into().unwrap()))
}

struct Message {
    client_id: u32,
    total_len: u32,
    data: String,
}

impl Message {
    fn new(client_id: u32, message: &str, total_len: u32) -> Self {
        Self {
            client_id,
            total_len,
            data: message.to_owned(),
        }
    }

    fn write(&self, fs: &mut FileSystem) {
        let mut file = fs.root_dir().create_file(&format!("{}.log", self.client_id)).unwrap();
        let pos: u32 = file.seek(SeekFrom::End(0)).unwrap().try_into().unwrap();

        if pos == 0 {
            let mut header = vec![0; 8];
            copy_bytes(&self.client_id.to_le_bytes(), &mut header, 0);
            copy_bytes(&self.total_len.to_le_bytes(), &mut header, 4);

            file.write_all(&header).unwrap();
        }

        let data_len: u32 = self.data.len().try_into().unwrap();
        let mut data = vec![0; self.total_len.try_into().unwrap()];

        copy_bytes(&self.client_id.to_le_bytes(), &mut data, 0);
        copy_bytes(&data_len.to_le_bytes(), &mut data, 4);
        copy_bytes(&self.data.as_bytes(), &mut data, 8);

        file.write_all(&data).unwrap();
        file.flush().unwrap();
    }
}

fn write_messages(client_count: u32, messages_per_client: u32, fs: &mut FileSystem) {
    let messages: Vec<Message> = (0..client_count)
        .map(|client| {
            Message::new(
                client,
                "Rust is cool!",
                32 * (1 << client), // break the A B C D pattern
            )
            // to keep it simple use cluster_size % block_size == 0
        })
        .collect();
    for _ in 0..messages_per_client {
        for message in &messages {
            message.write(fs);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MessageRecoveryBase {
    client_id: u32,
    data_len: usize,
}

impl MessageRecoveryBase {
    fn read(source: &[u8], offset: usize) -> Option<Self> {
        let client_id = read_u32(source, offset + 0);
        let data_len = read_u32(source, offset + 4);
        match (client_id, data_len) {
            (Some(client_id), Some(data_len)) => Some(Self {
                client_id,
                data_len: data_len.try_into().unwrap(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MessageRecovery {
    client_id: u32,
    block_len: usize,
    next_offset: usize,
    max_data_len: usize,
}

impl MessageRecovery {
    fn check(&self, base: &MessageRecoveryBase) -> ClusterBelongs {
        if base.client_id != self.client_id {
            return ClusterBelongs::NotToFile;
        }
        if base.data_len > self.max_data_len {
            return ClusterBelongs::NotToFile;
        }
        if base.data_len > self.block_len {
            return ClusterBelongs::NotToFile;
        }
        ClusterBelongs::ToFile
    }
}

impl Recovery for MessageRecovery {
    fn cluster_belongs_to_file(&mut self, cluster: &[u8]) -> ClusterBelongs {
        let base = MessageRecoveryBase::read(&cluster, self.next_offset);
        if let Some(base) = base {
            if base.client_id == 0 && base.data_len == 0 {
                return ClusterBelongs::NotToFile;
            }
            let first_check = self.check(&base);
            if first_check == ClusterBelongs::NotToFile {
                return ClusterBelongs::NotToFile;
            }
            self.next_offset += self.block_len;
            while let Some(next) = MessageRecoveryBase::read(&cluster, self.next_offset) {
                let next_check = self.check(&next);
                if next_check == ClusterBelongs::NotToFile {
                    return ClusterBelongs::IsEndOfFile;
                }
                self.next_offset += self.block_len;
            }
            self.next_offset = self.next_offset % cluster.len();
            return ClusterBelongs::ToFile;
        }
        ClusterBelongs::NotToFile
    }
}

struct MessageRecoveryFactory {
    max_client_id: u32,
    max_data_len: usize,
    max_block_len: usize,
}

impl RecoveryFactory for MessageRecoveryFactory {
    type State = MessageRecovery;

    fn is_start_of_file(&mut self, cluster: &[u8]) -> Option<Self::State> {
        let client_id = read_u32(cluster, 0).unwrap();
        let block_len: usize = read_u32(cluster, 4).unwrap().try_into().unwrap();
        if block_len > self.max_block_len || client_id > self.max_client_id {
            return None;
        }
        let mut tmp = MessageRecovery {
            client_id,
            next_offset: 8,
            block_len,
            max_data_len: self.max_data_len,
        };
        let check = tmp.cluster_belongs_to_file(cluster);
        match check {
            ClusterBelongs::NotToFile => None,
            ClusterBelongs::IsEndOfFile => Some(tmp),
            ClusterBelongs::ToFile => Some(tmp),
        }
    }
}

fn write_and_recover(mut fs: FileSystem) {
    write_messages(4, 17, &mut fs);
    let files = fs
        .recovery(
            MessageRecoveryFactory {
                max_client_id: 8,
                max_data_len: 256,
                max_block_len: 256,
            },
            35,
            65,
        )
        .unwrap();
    for file in files {
        let (data, state) = file.get_data();
        let mut file = fs.root_dir().open_file(&format!("{}.log", state.client_id)).unwrap();
        let mut original_data = vec![];
        file.read_to_end(&mut original_data).unwrap();
        let start_data = &data[0..original_data.len()];
        assert_eq!(start_data, original_data);
    }
}

#[test]
fn test_recovery_fat12() {
    call_with_fs(write_and_recover, FAT12_IMG, 1);
}

#[test]
fn test_recovery_fat16() {
    call_with_fs(write_and_recover, FAT16_IMG, 1);
}

#[test]
fn test_recovery_fat32() {
    call_with_fs(write_and_recover, FAT32_IMG, 1);
}
