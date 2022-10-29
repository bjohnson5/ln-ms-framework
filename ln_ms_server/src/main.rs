use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};
use ln_ms_lib::LnSimulation;

const HTML_STRING: &str = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Hello!</title></head><body><h1>Hello!</h1><p>Hi from Rust</p></body></html>";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Connection established!");
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let _http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // Setup the simulation
    let mut ln_sim = LnSimulation::new();
    ln_sim.create_node(String::from("blake"));
    ln_sim.create_node(String::from("brianna"));
    ln_sim.open_channel(String::from("blake"), String::from("brianna"), 500);
    ln_sim.create_node_online_event(String::from("blake"));
    ln_sim.create_node_offline_event(String::from("blake"));

    // Start the simulation
    ln_sim.run();

    let status_line = "HTTP/1.1 200 OK";
    let length = HTML_STRING.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{HTML_STRING}");
    stream.write_all(response.as_bytes()).unwrap();
}
