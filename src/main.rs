use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::{
    error::Error,
    fs::File,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tokio::net::TcpStream;

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

fn connect_to_ssh() -> Result<&'static str, &'static str> {
    for _ in 0..100 {
        if caller().is_ok() {
            return Ok("connected");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("error")
}

#[tokio::main]
async fn caller() -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect("127.0.0.1:2222").await?;
    println!("waiting for session");
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    println!("waiting for handshake");
    let s = sess.handshake().unwrap();
    //   sess.userauth_password("ubuntu", "pass").unwrap();
    Ok(())
}

fn exec_stream(
    disk: &str,
    seed: &str,
    file: &mut File,
) -> Result<(std::process::Child), Box<dyn Error>> {
    let comm = "qemu-system-x86_64";
    let mut cmd = Command::new(comm)
        .arg("-enable-kvm")
        .args(["-drive", &format!("file={disk},if=virtio")])
        .args(["-drive", &format!("file={seed},if=virtio,format=raw")])
        .args(["-netdev", "user,id=net00,hostfwd=tcp::2222-:22"])
        .args(["-device", "virtio-net-pci,netdev=net00"])
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
            println!("{}", l);
            if l.contains("ubuntu login:") {
                cmd.kill().expect("failed");
            }
        }
    }

    cmd.wait().unwrap();
    serde_json::to_writer(file, &data);
    Ok(cmd)
}

fn main() {
    //ssh connect!

    let disk = std::env::args().nth(1).unwrap();
    let seed = std::env::args().nth(2).unwrap();
    let disk_path = Path::new(&disk);

    let file_name = disk_path.file_name().unwrap();

    let path: String = format!("output/build-{:?}-{}.json", file_name, Utc::now());
    let mut log_file = File::create(path).expect("unable to create file");

    let start = Instant::now();
    println!("started ....");
    let cmd = exec_stream(&disk, &seed, &mut log_file);
    println!("about to ssh");

    match connect_to_ssh() {
        Ok(t) => {
            println!("connected at {:?}", t)
        }
        Err(e) => println!("error: {}", e),
    }
    cmd.unwrap().kill();
    let duration = start.elapsed();
    println!("Time elapsed is: {duration:?}");
}
