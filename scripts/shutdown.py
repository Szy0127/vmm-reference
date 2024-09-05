#!/usr/bin/python3
import socket

def main():
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

    client.connect("/tmp/rust-vmm.sock")

    message = "shutdown 0"
    client.sendall(message.encode('utf-8'))

    client.close()

if __name__ == "__main__":
    main()
