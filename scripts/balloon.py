#!/usr/bin/python3
import socket
import sys

def main():
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

    client.connect("/tmp/rust-vmm.sock")
    # 1024M: reclaim 1G
    # 0M: deflate, give back all
    M = int(sys.argv[1])
    pages = M * 256


    message = f"balloon {pages}"

    client.sendall(message.encode('utf-8'))

    client.close()

if __name__ == "__main__":
    main()
