// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::fs::File;
use std::result;

use log::warn;
use virtio_blk::request::Request;
use virtio_blk::stdio_executor::{self, StdIoBackend};
use virtio_queue::{DescriptorChain, Queue};
use vm_memory::{self, Bytes, GuestAddress,GuestAddressSpace, Address, GuestMemoryMmap, GuestMemory};

use crate::virtio::SignalUsedQueue;

#[derive(Debug)]
pub enum Error {
    GuestMemory(vm_memory::GuestMemoryError),
    Queue(virtio_queue::Error),
    ProcessRequest(stdio_executor::ProcessReqError),
}

impl From<vm_memory::GuestMemoryError> for Error {
    fn from(e: vm_memory::GuestMemoryError) -> Self {
        Error::GuestMemory(e)
    }
}

impl From<virtio_queue::Error> for Error {
    fn from(e: virtio_queue::Error) -> Self {
        Error::Queue(e)
    }
}

impl From<stdio_executor::ProcessReqError> for Error {
    fn from(e: stdio_executor::ProcessReqError) -> Self {
        Error::ProcessRequest(e)
    }
}

// This object is used to process the queue of a block device without making any assumptions
// about the notification mechanism. We're using a specific backend for now (the `StdIoBackend`
// object), but the aim is to have a way of working with generic backends and turn this into
// a more flexible building block. The name comes from processing and returning descriptor
// chains back to the device in the same order they are received.
pub struct SimpleHandler<M: GuestAddressSpace, S: SignalUsedQueue> {
    pub driver_notify: S,
    pub inflate: Queue<M>,
    pub deflate: Queue<M>,
    pub guest_mem: GuestMemoryMmap,
    pub inflate_page_num: u64
}

impl<M, S> SimpleHandler<M, S>
where
    M: GuestAddressSpace,
    S: SignalUsedQueue,
{
    fn inflate_page(&mut self, pfn:u32) -> result::Result<(), Error> {
        //println!("balloon process chain pfn {}", pfn);
        let gva = GuestAddress((pfn << 12).into());
        //TODO 
        //if let Some(region) = self.guest_mem.find_region(gva) {
            let hva = self.guest_mem.get_host_address(gva)
                .expect("get hva failed");

            //println!("hva {}", hva.unwrap() as u64);
            /*
            unsafe{
                println!("value:{}", *(hva.unwrap() as *const u64));
            }
            */
            let ret = unsafe{
                libc::madvise(hva.cast(), 4096, libc::MADV_DONTNEED)
            };
            if ret < 0 {
                println!("madvise failed");
            } else {
                self.inflate_page_num += 1;
                if self.inflate_page_num % 1000 == 0 {
                    println!("inflate page num {}", self.inflate_page_num);
                }
            }
        //}
        Ok(())
    }
    fn process_chain(&mut self, chain: &mut DescriptorChain<M::T>) -> result::Result<(), Error> {
        let mut buf:[u8;4] = [0;4];
        while let Some(desc) = chain.next() {
            let mut offset:u64 = 0;
            let len = desc.len() as u64;
            while offset < len {
                let addr = desc.addr().checked_add(offset).expect("address overflow");
                chain.memory()
                    .read_slice(&mut buf, addr)
                    .map_err(Error::GuestMemory)?;

                let pfn = u32::from_le_bytes(buf);
                self.inflate_page(pfn);

                offset += 4;

            }
            
        }

        Ok(())
    }

    pub fn process_queue(&mut self) -> result::Result<(), Error> {
        // To see why this is done in a loop, please look at the `Queue::enable_notification`
        // comments in `virtio_queue`.
        loop {
            self.inflate.disable_notification()?;

            while let Some(mut chain) = self.inflate.iter()?.next() {
                self.process_chain(&mut chain)?;
                self.inflate.add_used(chain.head_index(), 0)?;

                if self.inflate.needs_notification()? {
                    self.driver_notify.signal_used_queue(0);
                }
            }

            if !self.inflate.enable_notification()? {
                break;
            }
        }

        Ok(())
    }
}

// TODO: Figure out which unit tests make sense to add after implementing a generic backend
// abstraction for `InOrderHandler`.
