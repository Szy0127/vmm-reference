// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
use std::env;
use std::thread;
use std::io::{Read,Result};
use std::os::unix::net::{UnixListener,UnixStream};

use std::sync::{Arc, Mutex};
use api::Cli;
use vmm::{TryFrom1,Vmm, WrappedExitHandler};
use event_manager::{EventManager,MutEventSubscriber, SubscriberOps};


const sock_path:&str = "/tmp/rust-vmm.sock";
fn start_unix_socket_server(vmm: Arc<Mutex<Vmm>>) -> Result<()> {

    let listener = UnixListener::bind(sock_path).expect("create sock fail");
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let vmm = vmm.clone();
                    thread::spawn(move||{
                    let mut buffer = [0;1024];
                    match stream.read(&mut buffer) {
                        Ok(size) => {
                            let recv = String::from_utf8_lossy(&buffer[..size]);
                            let mut parts = recv.split_whitespace();
                            let (command, number_str) = match (parts.next(), parts.next()) {
                                (Some(cmd), Some(num_str)) => (cmd, num_str),
                                _ => {
                                    eprintln!("Failed to parse command and/or number");
                                    return;
                                }
                            };
                            let number: u64 = number_str.parse().unwrap();
                            match command {
                                "balloon" => {
                                    let mut vmm = vmm.lock().unwrap();
                                    vmm.change_balloon_config(number);
                                }
                                "shutdown" => {
                                    vmm.lock().unwrap().vm.shutdown();
                                    std::fs::remove_file(sock_path);
                                    std::process::exit(0);
                                }
                                cmd => {
                                    eprintln!("unkown {}", cmd);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("fail {}", e);
                        }
                    }
                    });
                }
                Err(e) => {
                    eprintln!("fail {}", e);
                }
            }
        }
    });
    Ok(())
}
fn main() {
    match Cli::launch(
        env::args()
            .collect::<Vec<String>>()
            .iter()
            .map(|s| s.as_str())
            .collect(),
    ) {
        Ok(vmm_config) => {
            let wrapped_exit_handler = WrappedExitHandler::new().expect("exit create failed");
            let mut event_manager = EventManager::<Arc<Mutex<dyn MutEventSubscriber + Send>>>::new()
                .expect("event create failed");
            event_manager.add_subscriber(wrapped_exit_handler.0.clone());
            let mut vmm =
                Arc::new(Mutex::new(Vmm::try_from1(vmm_config, &wrapped_exit_handler, &mut event_manager).expect("Failed to create VMM from configurations")));
            start_unix_socket_server(vmm.clone());
            // For now we are just unwrapping here, in the future we might use a nicer way of
            // handling errors such as pretty printing them.
            vmm.lock().unwrap().run().unwrap();
            loop {
                match event_manager.run() {
                   Ok(_) => (),
                   Err(e) => eprintln!("Failed to handle events: {:?}", e),
               }
               if !wrapped_exit_handler.keep_running() {
                   break;
               }
            }
            vmm.lock().unwrap().vm.shutdown();
        }
        Err(e) => {
            eprintln!("Failed to parse command line options. {}", e);
        }
    }
}
