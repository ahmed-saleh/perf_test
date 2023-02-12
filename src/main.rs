use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::process::{Command, Stdio};
use std::{
    fs::File,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize, Debug)]
struct Log {
    time: DateTime<Utc>,
    log_msg: String,
    duration: Duration,
}

impl Log {
    fn new(s: &str, d: Duration) -> Self {
        Log {
            time: Utc::now(),
            log_msg: s.to_string(),
            duration: d,
        }
    }
}

pub fn exec_stream(disk: &str, seed: &str, file: &mut File) {
    let comm = "qemu-system-x86_64";
    let mut cmd = Command::new(comm)
        .arg("-enable-kvm")
        .args(["-drive", &format!("file={},if=virtio", disk)])
        .args(["-drive", &format!("file={},if=virtio,format=raw", seed)])
        .args(["-device", "virtio-net-pci,netdev=net00"])
        .args(["-netdev", "type=user,id=net00"])
        .args(["-m", "512"])
        .arg("-nographic")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut data = vec![];
    {
        let stdout = cmd.stdout.as_mut().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        for line in stdout_lines {
            data.push(Log::new(&line.unwrap()));
        }
    }

    cmd.wait().unwrap();

    // write log
    for i in &data {
        file.write_all(format!("{}", serde_json::to_string(i).unwrap()).as_bytes());
    }
}

fn main() {
    let disk = std::env::args().nth(1).unwrap();
    let seed = std::env::args().nth(2).unwrap();
    let disk_path = Path::new(&disk);
    let file_name = disk_path.file_name().unwrap();

    let path: String = format!("/output/build-{:?}-{}.csv", file_name, Utc::now());
    let mut log_file = File::create(&path).expect("unable to create file");

    exec_stream(&disk, &seed, &mut log_file);
}
