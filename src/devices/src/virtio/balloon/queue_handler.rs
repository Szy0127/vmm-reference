// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use event_manager::{EventOps, Events, MutEventSubscriber};
use log::error;
use vm_memory::{GuestAddressSpace, GuestMemory};
use vmm_sys_util::epoll::EventSet;
use vmm_sys_util::eventfd::EventFd;

use crate::virtio::balloon::simple_handler::SimpleHandler;
use crate::virtio::SingleFdSignalQueue;

const INFLATE_IOEVENT_DATA: u32 = 0;
const DEFLATE_IOEVENT_DATA: u32 = 1;

// This object simply combines the more generic `SimpleHandler` with a concrete queue
// signalling implementation based on `EventFd`s, and then also implements `MutEventSubscriber`
// to interact with the event manager. `ioeventfd` is the `EventFd` connected to queue
// notifications coming from the driver.
pub(crate) struct QueueHandler<M: GuestAddressSpace> {
    pub inner:SimpleHandler<M, SingleFdSignalQueue>,
    pub inflate_io: EventFd,
    pub deflate_io: EventFd,
}

impl<M: GuestAddressSpace> MutEventSubscriber for QueueHandler<M> {
    fn process(&mut self, events: Events, ops: &mut EventOps) {
        let mut error = true;

        // TODO: Have a look at any potential performance impact caused by these conditionals
        // just to be sure.
        if events.event_set() != EventSet::IN {
            error!("unexpected event_set");
        } 
        match events.data() {
            INFLATE_IOEVENT_DATA => {
                if self.inflate_io.read().is_err() {
                    error!("ioeventfd read error")
                } else if let Err(e) = self.inner.process_inflate() {
                    error!("error processing block queue {:?}", e);
                } else {
                    error = false;
                }
            }

            DEFLATE_IOEVENT_DATA => {
                if self.deflate_io.read().is_err() {
                    error!("ioeventfd read error")
                } else if let Err(e) = self.inner.process_deflate() {
                    error!("error processing block queue {:?}", e);
                } else {
                    error = false;
                }
            }

            data => {
                error!("unexpected events data {}", data);
            }
        }

        if error {
            ops.remove(events)
                .expect("Failed to remove fd from event handling loop");
        }
    }

    fn init(&mut self, ops: &mut EventOps) {
        ops.add(Events::with_data(
            &self.inflate_io,
            INFLATE_IOEVENT_DATA,
            EventSet::IN,
        ))
        .expect("Failed to init inflate queue handler");
        ops.add(Events::with_data(
            &self.deflate_io,
            DEFLATE_IOEVENT_DATA,
            EventSet::IN,
        ))
        .expect("Failed to init deflate queue handler");
    }
}

// TODO: Figure out if unit tests make sense here as well after implementing a generic backend
// abstraction for the `SimpleHandler`.
