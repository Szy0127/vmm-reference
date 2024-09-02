// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
use std::convert::TryFrom;
use std::env;
use std::thread;
use std::io::{Read,Result};
use std::os::unix::net::{UnixListener,UnixStream};

use std::sync::{Arc, Mutex};
use api::Cli;
use vmm::Vmm;


/*
fn start_unix_socket_server(vmm: Arc<Mutex<Vmm>>) -> Result<()> {

    let listener = UnixListener::bind("/tmp/rust-vmm.sock").expect("create sock fail");
    println!("start unix");
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
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
                            println!("cmd:{}, num:{}", command, number);
                            let mut vmm = vmm.lock().unwrap();
                            vmm.change_balloon_config(number);
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
*/
fn main() {
    match Cli::launch(
        env::args()
            .collect::<Vec<String>>()
            .iter()
            .map(|s| s.as_str())
            .collect(),
    ) {
        Ok(vmm_config) => {
            let mut vmm =
                Vmm::try_from(vmm_config).expect("Failed to create VMM from configurations");
            //start_unix_socket_server(Arc::new(Mutex::new(vmm)));
            // For now we are just unwrapping here, in the future we might use a nicer way of
            // handling errors such as pretty printing them.
            vmm.run().unwrap();
        }
        Err(e) => {
            eprintln!("Failed to parse command line options. {}", e);
        }
    }
}
