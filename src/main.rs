use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
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
        .args(["-drive", &format!("file={disk},if=virtio")])
        .args(["-drive", &format!("file={seed},if=virtio,format=raw")])
        .args(["-device", "virtio-net-pci,netdev=net00"])
        .args(["-netdev", "type=user,id=net00"])
        .args(["-m", "512"])
        .arg("-nographic")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut base_time = Instant::now();
    let mut data = vec![];
    {
        let stdout = cmd.stdout.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        for line in stdout_lines {
            let l = line.unwrap();
            let duration = base_time.elapsed();
            base_time = Instant::now();
            data.push(Log::new(&l, duration));
            //
            //the closest to kill switch
            if l.contains("Ubuntu ") && l.contains(" LTS ubuntu ttyS0") {
                cmd.kill().expect("failed");
            }
        }
    }

    cmd.wait().unwrap();
    serde_json::to_writer(file, &data);
}

fn main() {
    let disk = std::env::args().nth(1).unwrap();
    let seed = std::env::args().nth(2).unwrap();
    let disk_path = Path::new(&disk);
    let file_name = disk_path.file_name().unwrap();

    let path: String = format!("output/build-{:?}-{}.json", file_name, Utc::now());
    let mut log_file = File::create(path).expect("unable to create file");

    let start = Instant::now();
    println!("started ....");
    exec_stream(&disk, &seed, &mut log_file);

    let duration = start.elapsed();
    println!("Time elapsed is: {duration:?}");
}
