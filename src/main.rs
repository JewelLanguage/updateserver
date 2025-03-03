use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    env,
    fmt
};
use std::fmt::Debug;
use std::path::Path;
use std::sync::mpsc::channel;
use random_string::generate;
use serde::{Serialize, Deserialize};
use serde_json;


#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct Version{
    major:i32,
    minor:i32,
    build:i32,
    patch:i32,
    count:i32   // Number of successful downloads of this version
}

impl fmt::Display for Version{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Versions{
    dev:Vec<Version>,
    stable:Vec<Version>,
    beta:Vec<Version>,
    canary:Vec<Version>,
    extended:Vec<Version>
}

impl fmt::Display for Versions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Versions:\n")?;
        write!(f, "  Dev:    {:?}\n", self.dev)?;
        write!(f, "  Stable: {:?}\n", self.stable)?;
        write!(f, "  Beta:   {:?}\n", self.beta)?;
        write!(f, "  Canary: {:?}\n", self.canary)?;
        write!(f, "  Extended: {:?}\n", self.extended)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown
}

#[derive(Serialize, Deserialize)]
enum Architecture{
    Arm,
    Arm64,
    x86,
    x86_64,
    x64
}

#[derive(Serialize, Deserialize)]
struct Hardware{
    sse:i32,
    sse2:i32,
    sse41:i32,
    sse42:i32,
    sse3:i32,
    avx:i32,
    physmemory:i32 // Physical memory available to the client, if unknown, assume its the size of the latest release +2GB
}

#[derive(Serialize, Deserialize)]
struct OperatingSystem{
    platform:String,
    sp:String, // Service Pack
    arch:String,
    dedup:String, // used to dedup user count
}

#[derive(Serialize, Deserialize)]
enum Channel{
    Stable,
    Beta,
    Dev,
    Canary,
    Extended
}

impl fmt::Display for Channel{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channelString = match self{
            Channel::Stable => "Stable",
            Channel::Beta => "Beta",
            Channel::Dev => "Dev",
            Channel::Canary => "Canary",
            Channel::Extended => "Extended",
        };
        write!(f, "{}", channelString);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Request{
    updater:String, // client software
    acceptformat:String,
    hw:Hardware,
    ismachine:i32, // used system-wide or only for a single user
    os:OperatingSystem,
    protocol:f32, // the version of mini omaha protocol
    requestid:String,
    sessionid:String,
    channel:Channel,
    updaterversion:f32
}

#[derive(Serialize, Deserialize)]
struct TimerObject{
    elapsed_days:i32
}

#[derive(Serialize, Deserialize)]
struct SysRequirements{
    platform:Platform,
    arch:Architecture,
    min_os_version:f32,
    server:String
}

#[derive(Serialize, Deserialize)]
struct Status{
    ok:String,
    error:i32 // 1 -> invalid arguments, 2 - Not found
}

#[derive(Serialize, Deserialize)]
struct Response {
    daystart: TimerObject,
    name: String,
    status: Status
}
fn handle_connection(mut stream: TcpStream, versions:&Versions){
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() {
        eprintln!("Failed to read request line");
        return;
    }

    println!("Request Line: {}", request_line.trim());

    let mut content_length = 0;
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" {
            break;
        }
        if line.starts_with("Content-Length:") {
            content_length = line
                .trim()
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
        }
        headers.push_str(&line);
    }

    println!("Headers:\n{}", headers);

    let mut body = String::new();
    if content_length > 0 {
        let mut buffer = vec![0; content_length];
        if reader.read_exact(&mut buffer).is_ok() {
            body = String::from_utf8_lossy(&buffer).to_string();
        }
    }

    println!("Body:\n{}", body);

    parse_request(request_line,body, versions);

    handle_okresponse(stream);

    println!("Response sent!");
}

fn generate_id() -> String {
    let character_set ="0123456789abcdefghijklmnopqrstuvwxyz";
    let generated_id = generate(25,character_set); // 128 bits of entropy
    generated_id
}

fn handle_okresponse(mut stream: TcpStream){
    let ok_response = "HTTP/1.1 200 OK\r\n";
    let contents = fs::read_to_string("index.html").unwrap();
    let length = contents.len();

    let entire_body =format!("{ok_response}Content-Length: {length}\r\n\r\n{contents}");
    stream.write_all(entire_body.as_bytes()).unwrap();
}

fn parse_request(request_header:String, body:String, versions:&Versions) {
    let split_line = request_header.split(" ").collect::<Vec<&str>>();
    let method = split_line[0];
    let endpoint = split_line[1];

    match method {
        "GET" => {
            println!("Incoming GET request");
        },
        "POST" => {
            println!("Incoming POST request");
        },
        _ => {
            return;
        }
    }

    match endpoint {
        "/latest" => {  // equivalent of update-check
            let mut request_data = serde_json::from_str::<Request>(&body.as_str()).unwrap();

            if(request_data.sessionid == ""){
                request_data.sessionid = generate_id();
            }

            if(request_data.sessionid == ""){
                request_data.sessionid = generate_id();
            }

            match request_data.channel {
                Channel::Dev => {
                    println!("Latest {}",versions.dev.first().unwrap());
                },
                _ => {
                    println!("Channel {} not supported", request_data.channel)
                }
            }
        },
        "/download" => {    // the download phase/ping check

        },
        "status" => {   // equivalent of ping-bacl

        },
        _ => {
            return;
        }
    }
}

fn main() {

    let versions_path= Path::new("versions.json");
    let versions_content = fs::read_to_string(versions_path)
        .expect("Should have been able to read the file");

    let versions = serde_json::from_str::<Versions>(&versions_content).unwrap();
    println!("All Versions : {}",versions);

    let listener = TcpListener::bind("127.0.0.1:7778").unwrap();
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        handle_connection(stream, &versions);
    }
}



