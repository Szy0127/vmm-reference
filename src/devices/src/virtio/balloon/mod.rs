// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

mod device;
mod queue_handler;
mod simple_handler;

use vm_memory::GuestMemoryMmap;
pub use device::Balloon;

// TODO: Move relevant defines to vm-virtio crate.

// Values taken from the virtio standard (section 5.1.3 of the 1.1 version).
pub mod features {
    pub const VIRTIO_F_VERSION_2: u64 = 32;
}


// Net device ID as defined by the standard.
pub const BALLOON_DEVICE_ID: u32 = 5;



#[derive(Debug)]
pub enum Error {
    Virtio(crate::virtio::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct BalloonArgs {
    pub guest_memory: GuestMemoryMmap
}
