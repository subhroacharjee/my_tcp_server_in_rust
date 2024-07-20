use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::usize;

pub fn create_tcp_listener_or_panic(port: &str) -> TcpListener {
    let addr = format!("127.0.0.1:{}", port);
    match TcpListener::bind(addr) {
        Ok(lt) => lt,
        Err(err) => {
            println!("failed to create a server. Err {}", err);
            panic!();
        }
    }
}

pub fn handle_stream(
    stream: TcpStream,
    counter: usize,
    server_context: Arc<Mutex<HashMap<usize, TcpStream>>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        let mut buffer = BufReader::new(stream.try_clone().unwrap());
        let mut message = String::new();
        if buffer.read_line(&mut message).unwrap() > 0 {
            println!("read [{}]> {}", counter, message);
        } else {
            let mut stream_maps = server_context.lock().unwrap();

            stream_maps.remove(&counter);
            break;
        }
    })
}

pub fn broadcast_message_to_all_streams(
    server_context: Arc<Mutex<HashMap<usize, TcpStream>>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        let mut broadcast_message = String::new();
        io::stdin().read_line(&mut broadcast_message).unwrap();
        broadcast_message = format!("server> {}", broadcast_message);

        // we are creating a scope for mutex guard so that as soon as scope is resolved it is
        // dropped.
        {
            let stream_map = server_context.lock().unwrap();
            for stream in stream_map.values() {
                let mut writer_stream: TcpStream = stream.try_clone().unwrap();
                match writer_stream.write(broadcast_message.as_bytes()) {
                    Ok(_s) => {}
                    Err(err) => {
                        println!("err {}", err);
                    }
                };
            }
        }
    })
}

pub fn run() -> std::io::Result<()> {
    let listener = create_tcp_listener_or_panic("7878");
    let stream_map_mutex = Arc::new(Mutex::new(HashMap::new()));
    let mut handlers = vec![];

    handlers.push(broadcast_message_to_all_streams(Arc::clone(
        &stream_map_mutex,
    )));

    for (counter, stream) in listener.incoming().enumerate() {
        match stream {
            Ok(stream) => {
                {
                    let context = Arc::clone(&stream_map_mutex);
                    let mut map = context.lock().unwrap();
                    map.insert(counter, stream.try_clone().unwrap());
                }
                handlers.push(handle_stream(
                    stream,
                    counter,
                    Arc::clone(&stream_map_mutex),
                ));
            }
            Err(err) => {
                println!("err {}", err);
            }
        }
    }

    for handler in handlers {
        handler.join().unwrap();
    }

    drop(listener);
    Ok(())
}
