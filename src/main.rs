use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    env,
    fmt
};
use std::backtrace::BacktraceStatus;
use std::fmt::Debug;
use std::path::Path;
use std::sync::mpsc::channel;
use random_string::generate;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::Action::{abandon, retry};
use crate::Status::noupdate;
// use serde_json::Value::String;

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct Version{
    major:i32,
    minor:i32,
    build:i32,
    patch:i32,
    count:i32,   // Number of successful downloads of this version
    urls:Vec<String>,
}

impl fmt::Display for Version{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}, {:?}", self.major, self.minor, self.build, self.urls)?;
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
enum Status{
    ok,
    noupdate,
    errorinternal,
    errorhash,
    errorosnotsupported,
    errorhwnotsupported,
    errorunsupportedprotocol,
}

#[derive(Serialize, Deserialize)]
struct Manifest{
    arguments:String,
    run:String, // basically the installer (this will need work)
    version:Version,
    url:String // the download url for the new version
}

#[derive(Serialize, Deserialize)]
struct Response {
    daystart: TimerObject,
    name: String,
    status: Status,
    manifest:Manifest,
}



// List of actions that can be taken based on specific response
// Only two actions supported right now:
// download -> download after verification
// abandon -> no more responses
#[derive(Serialize, Deserialize)]
enum Action{
    download,
    abandon,
    retry
}

#[derive(Serialize, Deserialize)]
struct LatestResponse{
    actions:Vec<Action>,
    info:String,
    status:Status,
    version: String,
    sessionid:String,
    requestid:String
}

fn handle_connection(mut stream: TcpStream, versions:&Versions){
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() {
        println!("Failed to read request line");
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

    parse_request(stream,request_line,body, versions);

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

fn create_response(status_code:i32, message:&str) -> String{
    let status_string = match status_code {
        // Informational responses (100–199)
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",

        // Successful responses (200–299)
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",

        // Redirection messages (300–399)
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",

        // Client error responses (400–499)
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",

        // Server error responses (500–599)
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "Unknown Status Code",
    };

    let response_string = format!("HTTP/1.1 {status_code} {status_string}\r\n");
    let response_content  = format!("{response_string}Content-Type: application/json\r\n");
    let contents = String::from(message);
    let length = contents.len();
    let entire_body =format!("{response_content}Content-Length: {length}\r\n\r\n{contents}");

    entire_body
}

fn handle_latest_response(mut stream: &TcpStream, version:&Version, status:Status, request: &Request){
    let mut response_string = String::from("");
    let mut response_object  = LatestResponse {
        actions: vec![],
        info:String::from("server prototype"),
        status:Status::noupdate,
        version: version.to_string(),
        sessionid:request.sessionid.to_string(),
        requestid:request.requestid.to_string()
    };

    if(request.requestid == "" || request.sessionid == ""){
        response_string = create_response(500, &serde_json::to_string(&response_string).unwrap());
        stream.write_all(response_string.as_bytes()).unwrap();
        return
    }

    match status {
        Status::ok => {
            let actions = vec![Action::download, Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::ok;
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap());
        },
        Status::noupdate => {
            let actions = vec![];
            response_object.actions = actions;
            response_object.status = Status::noupdate;
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorinternal=> {
            let actions = vec![Action::retry,Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::errorinternal;
            response_string = create_response(500,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorosnotsupported =>{
            let actions = vec![Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::errorosnotsupported;
            response_string = create_response(406,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorhwnotsupported=> {
            let actions = vec![Action::download, Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::errorhwnotsupported;
            response_string = create_response(428,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorunsupportedprotocol=> {
            let actions = vec![Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::errorunsupportedprotocol;
            response_string = create_response(401, &serde_json::to_string(&response_object).unwrap());
        },
        _ => {
            let actions = vec![Action::abandon];
            response_object.actions = actions;
            response_object.status = Status::noupdate;
            response_string = create_response(400, &serde_json::to_string(&response_object).unwrap());
        }
    }

    stream.write_all(response_string.as_bytes()).unwrap();
}

fn parse_request(mut stream: TcpStream,request_header:String, body:String, versions:&Versions) {
    let default_version = Version{
        major:0,
        minor:0,
        build:0,
        patch:0,
        count:0,
        urls:vec![]
    };

    let default_request = Request{
        updater:String::from(""), // client software
        acceptformat:String::from(""),
        hw: Hardware {
            sse:-1,
            sse2:-1,
            sse41:-1,
            sse42:-1,
            sse3:-1,
            avx:-1,
            physmemory:-1
        },
        ismachine:0, // used system-wide or only for a single user
        os: OperatingSystem{
            platform:String::from(""),
            sp:String::from(""), // Service Pack
            arch:String::from(""),
            dedup:String::from(""), // used to dedup user count
        },
        protocol:1.0, // the version of mini omaha protocol
        requestid:String::from(""),
        sessionid:String::from(""),
        channel: Channel::Dev,
        updaterversion:0.0
    };

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
            let mut request_data = if body == "" { default_request } else { serde_json::from_str::<Request>(&body.as_str()).unwrap() };

            if(request_data.sessionid == ""){
                request_data.sessionid = generate_id();
            }

            if(request_data.requestid == ""){
                request_data.requestid = generate_id();
            }

            match request_data.channel {
                Channel::Dev => {
                    handle_latest_response(&stream, versions.dev.first().unwrap(),Status::ok, &request_data);
                },
                _ => {
                    println!("Channel {} not supported", request_data.channel);
                    handle_latest_response(&stream, &default_version ,Status::noupdate, &request_data);
                }
            }
        },

        "/download" => {    // the download phase/ping check
        },
        "/status" => {   // equivalent of ping-back
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



