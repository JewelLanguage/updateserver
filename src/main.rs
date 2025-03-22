mod version;
mod latest;
mod session;

use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    fmt
};
use std::fmt::Debug;
use std::path::Path;
use std::thread::current;
use random_string::generate;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::session::{new_session, remove_session, update_current_action, update_request, update_session_actions, Session_Manager};
use crate::version::Version;

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

#[derive(Serialize, Deserialize, PartialEq,Clone)]
struct Hardware{
    sse:i32,
    sse2:i32,
    sse41:i32,
    sse42:i32,
    sse3:i32,
    avx:i32,
    physmemory:i32 // Physical memory available to the client, if unknown, assume its the size of the latest release +2GB
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct OperatingSystem{
    platform:String,
    sp:String, // Service Pack
    arch:String,
    dedup:String, // used to dedup user count
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
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
    updaterversion:f32,

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
enum Status{    // Used in status requests/checks for a particular session
    ok,
    noupdate,
    errorinternal,
    errorhash,
    errorosnotsupported,
    errorhwnotsupported,
    errorunsupportedprotocol,
    updatecomplete,
    updateabandoned,
}


#[derive(Serialize, Deserialize)]
struct Manifest{
    arguments:String,
    run:String, // basically the installer (this will need work)
    version:version::Version,
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
// abandon -> no more responses,
// retry -> for failures
#[derive(Serialize, Deserialize,PartialEq, Clone)]
enum Action{
    download,
    abandon,
    retry,
    latest,
    complete,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
enum EventType {
    Install,
    Update,
    Uninstall,
    Download,
    Complete,
    None
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

#[derive(Serialize, Deserialize)]
struct DownloadResponse{
    actions:Vec<Action>,
    info:String,
    status:Status,
    sessionid:String,
    requestid:String,
    downloadlink:String
}

#[derive(Serialize, Deserialize)]
struct StatusResponse{
    sessionid:String,
    requestid:String,
    status:Status,
}


#[derive(Serialize, Deserialize)]
struct StatusRequest{
    request:Request,
    eventtype:EventType,
    action:Action,
    result:i32, // 0 => error, 1 => success, 2 => cancelled
}

fn handle_connection(mut stream: TcpStream, versions:&version::Versions, session_manager:&mut Session_Manager){
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

    let mut body = String::new();
    if content_length > 0 {
        let mut buffer = vec![0; content_length];
        if reader.read_exact(&mut buffer).is_ok() {
            body = String::from_utf8_lossy(&buffer).to_string();
        }
    }

    println!("Body:\n{}", body);

    parse_request(stream,request_line,body, versions, session_manager);

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


fn handle_latest_response(mut stream: &TcpStream, version:&version::Version, status:Status, request: &Request){
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



fn handle_download_response(mut stream: &TcpStream, version:&version::Version, status:Status, request: &Request, session_manager:&mut Session_Manager){
    let mut response_string = String::from("");


    let mut response_object = DownloadResponse{
        actions: vec![],
        info:String::from("server prototype"),
        status:Status::noupdate,
        sessionid:request.sessionid.to_string(),
        requestid:request.requestid.to_string(),
        downloadlink : version.urls.first().unwrap().clone()
    };

    if(request.requestid == "" || request.sessionid == ""){
        response_string = create_response(500, &serde_json::to_string(&response_string).unwrap());
        stream.write_all(response_string.as_bytes()).unwrap();
        return
    }

    match status {
        Status::ok => {
            let actions = session_manager.sessions.get(&request.sessionid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::ok;
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap())
        },
        Status::noupdate => {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::noupdate;
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorinternal=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::errorinternal;
            response_string = create_response(500,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorosnotsupported =>{
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::errorosnotsupported;
            response_string = create_response(406,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorhwnotsupported=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::errorhwnotsupported;
            response_string = create_response(428,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorunsupportedprotocol=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::errorunsupportedprotocol;
            response_string = create_response(401, &serde_json::to_string(&response_object).unwrap());
        },
        _ => {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.actions = actions;
            response_object.status = Status::noupdate;
            response_string = create_response(400, &serde_json::to_string(&response_object).unwrap());
        }
    }

    stream.write_all(response_string.as_bytes()).unwrap();
}


fn handle_status_response(mut stream: &TcpStream, status:Status,version:&version::Version, request: &Request, session_manager:&mut Session_Manager){
    let mut response_string = String::from("");
    let mut response_object = StatusResponse{
        sessionid:request.clone().sessionid,
        requestid:request.clone().sessionid,
        status:Status::ok
    };

    if(request.requestid == "" || request.sessionid == ""){
        response_string = create_response(500, &serde_json::to_string(&response_string).unwrap());
        stream.write_all(response_string.as_bytes()).unwrap();
        return
    }

    match status {
        Status::ok => {
            let actions = session_manager.sessions.get(&request.sessionid).unwrap().possible_actions.clone();
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap())
        },
        Status::noupdate => {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::noupdate;
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorinternal=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::errorinternal;
            response_string = create_response(500,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorosnotsupported =>{
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::errorosnotsupported;
            response_string = create_response(406,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorhwnotsupported=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::errorhwnotsupported;
            response_string = create_response(428,&serde_json::to_string(&response_object).unwrap());
        },
        Status::errorunsupportedprotocol=> {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::errorunsupportedprotocol;
            response_string = create_response(401, &serde_json::to_string(&response_object).unwrap());
        },
        Status::updateabandoned => {
            let actions = session_manager.sessions.get(&request.sessionid).unwrap().possible_actions.clone();
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap())
        }
        Status::updatecomplete => {
            let actions = session_manager.sessions.get(&request.sessionid).unwrap().possible_actions.clone();
            response_string = create_response(200,&serde_json::to_string(&response_object).unwrap())
        },
        _ => {
            let actions = session_manager.sessions.get(&request.requestid).unwrap().possible_actions.clone();
            response_object.status = Status::noupdate;
            response_string = create_response(400, &serde_json::to_string(&response_object).unwrap());
        }
    }
    stream.write_all(response_string.as_bytes()).unwrap();
}



fn parse_request(mut stream: TcpStream,request_header:String, body:String, versions:&version::Versions, session_manager:&mut Session_Manager) {
    let default_version = version::Version{
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
            if(method != "GET"){
                handle_latest_response(&stream, &default_version, Status::errorunsupportedprotocol, &default_request);
                return;
            }

            let mut request_data = if body == "" { default_request } else { serde_json::from_str::<Request>(&body.as_str()).unwrap() };
            handle_latest(&stream, &default_version, versions,session_manager, &mut request_data);
        },

        "/download" => {    // the download phase/ping check
            if(method != "GET"){
                handle_download_response(&stream, &default_version, Status::errorunsupportedprotocol, &default_request, session_manager);
                return;
            }

            let mut request_data = if body == "" { default_request } else { serde_json::from_str::<Request>(&body.as_str()).unwrap() };
            handle_download(&stream, &default_version, versions,session_manager, &mut request_data);
        },

        "/status" => {   // equivalent of ping-back
            if(method != "GET"){
                return;
            }

            let mut default_status_request = StatusRequest{
                eventtype:EventType::None,
                result:1,
                action:Action::retry,
                request:default_request.clone()
            };

            let mut request_data = if body == "" { default_status_request } else { serde_json::from_str::<StatusRequest>(&body.as_str()).unwrap() };
            if(session_manager.sessions.contains_key(&request_data.request.sessionid)){
                let current_session = session_manager.sessions.get(&request_data.request.sessionid).unwrap().clone();
                if(current_session.possible_actions.contains(&request_data.action)){
                    match request_data.result{
                        0 => {
                            handle_status_action(&stream,&default_version, versions, session_manager, &mut request_data.request, &request_data.action, &current_session.previous_action);
                        },
                        1 => {
                            session_manager.sessions.remove(&request_data.request.sessionid);
                            handle_status_response(&stream, Status::ok, &default_version,&request_data.request, session_manager);
                        }
                        2 => {
                            handle_status_action(&stream,&default_version, versions, session_manager, &mut request_data.request, &request_data.action, &current_session.previous_action);
                        }
                        _ => {
                            handle_status_response(&stream, Status::errorinternal, &default_version,&request_data.request, session_manager);
                        }
                    }
                }
            }
        },
        _ => {
            return;
        }
    }
}


fn handle_latest(mut stream: &TcpStream,default_version:&Version, versions:&version::Versions, session_manager:&mut Session_Manager, request_data:&mut Request){
    if(request_data.sessionid == ""){
        request_data.sessionid = generate_id();
    }

    if(!new_session(session_manager, &request_data)){
        println!("Failed to create a new session because session already exists");
        return
    }

    if(request_data.requestid == ""){
        let new_request_id = generate_id();
        request_data.requestid = new_request_id.clone();
        update_request(session_manager, &request_data, new_request_id);
        update_current_action(session_manager, &request_data, Action::latest);
        update_session_actions(session_manager, &request_data, vec![Action::latest,Action::download, Action::abandon, Action::retry]);
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
}

fn handle_download(stream: &TcpStream, default_version:&Version, versions:&version::Versions, session_manager:&mut Session_Manager, request_data:&mut Request){
    if(session_manager.sessions.contains_key(&request_data.sessionid)){
        let current_session = session_manager.sessions.get(&request_data.sessionid).unwrap();
        if(current_session.requestid == request_data.requestid && current_session.previous_action == Action::latest && current_session.possible_actions.contains(&Action::download)){
            let new_request_id = generate_id();
            let previous_action = Action::download;
            let update_request_result = update_request(session_manager, &request_data, new_request_id.clone());
            if(update_request_result.0){
                request_data.requestid = new_request_id;
                let update_current_action = update_current_action(session_manager, &request_data, previous_action);
                if(update_current_action.0){
                    let update_session_actions = update_session_actions(session_manager, &request_data, vec![Action::abandon, Action::retry]);
                    if(update_session_actions.0){
                        // all data is updated, create and send response
                        match request_data.channel {
                            Channel::Dev => {
                                handle_download_response(&stream, versions.dev.first().unwrap(),Status::ok, &request_data,session_manager);
                                return;
                            },
                            _ => {
                                println!("Channel {} not supported", request_data.channel);
                                handle_download_response(&stream, &default_version ,Status::noupdate, &request_data,session_manager);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    handle_download_response(&stream, &default_version ,Status::noupdate, &request_data,session_manager);
    return;
}

fn handle_status_action(stream: &TcpStream, default_version:&Version, versions:&version::Versions, session_manager:&mut Session_Manager, request_data:&mut Request, action:&Action, previous_action:&Action){
    match action {
        Action::retry => {
            // we could store the last response and send it again??
            match previous_action {
                Action::latest => {
                    handle_latest(&stream,&default_version, versions, session_manager, request_data );
                },
                Action::download => {
                    handle_download(&stream, &default_version, versions, session_manager, request_data);
                },
                _ => {
                    handle_status_response(&stream, Status::errorunsupportedprotocol,default_version,request_data, session_manager)
                }
            }
        },
        Action::abandon => {
            session_manager.sessions.remove(&request_data.sessionid);   // we delete your session and send back a success response
        },
        Action::complete => {
            session_manager.sessions.remove(&request_data.sessionid);   // clear session and send back response
        }
        _ => {
            handle_status_response(&stream, Status::errorunsupportedprotocol,default_version,request_data, session_manager)
        }
    }
}

fn handle_status(stream: &TcpStream, default_version:&Version, versions:&version::Versions, session_manager:&mut Session_Manager, request_data:&mut StatusRequest){
    // cloning let us use the session manager without running into the error below:
    // cannot borrow *session_manager as mutable because it is also borrowed as immutable
    let session_manager_cpy = session_manager.sessions.clone(); // cloning allows us to avoid the

    if(session_manager.sessions.contains_key(&request_data.request.sessionid)){
        let current_session = session_manager_cpy.get(&request_data.request.sessionid).unwrap();
        if(current_session.possible_actions.contains(&request_data.action)){
            match request_data.result{
                0 => {
                    handle_status_action(&stream,&default_version, versions, session_manager, &mut request_data.request, &request_data.action, &current_session.previous_action);
                },
                1 => {
                    session_manager.sessions.remove(&request_data.request.sessionid);
                    handle_status_response(&stream, Status::ok, &default_version,&request_data.request, session_manager);
                }
                2 => {
                    handle_status_action(&stream,&default_version, versions, session_manager, &mut request_data.request, &request_data.action, &current_session.previous_action);
                }
                _ => {
                    handle_status_response(&stream, Status::errorinternal, &default_version,&request_data.request, session_manager);
                }
            }
        }
    }

    handle_status_response(&stream, Status::errorunsupportedprotocol, &default_version, &request_data.request, session_manager);
}

fn main() {

    let versions_path= Path::new("versions.json");
    let versions_content = fs::read_to_string(versions_path)
        .expect("Should have been able to read the file");

    let mut session_manager = session::new_session_manager();

    let versions = serde_json::from_str::<version::Versions>(&versions_content).unwrap();
    println!("All Versions : {}",versions);

    let listener = TcpListener::bind("127.0.0.1:7778").unwrap();
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        handle_connection(stream, &versions, &mut session_manager);
    }
}



