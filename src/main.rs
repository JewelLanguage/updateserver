use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7778").unwrap();
    for stream in listener.incoming(){
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream){
    let buf_reader = BufReader::new(&stream);
    let http_request:Vec<_> = buf_reader
       .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {http_request:#?}");

    handle_okresponse(stream);

    println!("Response sent!");

}


fn handle_okresponse(mut stream: TcpStream){
    let ok_response = "HTTP/1.1 200 OK\r\n";
    let contents = fs::read_to_string("index.html").unwrap();
    let length = contents.len();

    let entire_body =format!("{ok_response}Content-Length: {length}\r\n\r\n{contents}");
    stream.write_all(entire_body.as_bytes()).unwrap();
}

