// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::borrow::{Borrow, BorrowMut};
use std::fs::OpenOptions;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{Ordering};

use virtio_blk::stdio_executor::StdIoBackend;
use virtio_device::{VirtioConfig, VirtioDeviceActions, VirtioDeviceType, VirtioMmioDevice};
use virtio_queue::Queue;
use vm_device::bus::MmioAddress;
use vm_device::device_manager::MmioManager;
use vm_device::{DeviceMmio, MutDeviceMmio};
use vm_memory::{GuestAddressSpace, GuestMemoryMmap};

use crate::virtio::{CommonConfig, Env, SingleFdSignalQueue, QUEUE_MAX_SIZE};
use crate::virtio::features::{VIRTIO_F_VERSION_1};
use crate::virtio::balloon::{BALLOON_DEVICE_ID};

use super::simple_handler::SimpleHandler;
use super::queue_handler::QueueHandler;
use super::{BalloonArgs, Error, Result};

const VIRTIO_MMIO_INT_VRING :u8 = 1 << 0;
const VIRTIO_MMIO_INT_CONFIG :u8 = 1 << 1;


pub struct Balloon<M: GuestAddressSpace> {
    pub cfg: CommonConfig<M>,
    pub guest_memory: GuestMemoryMmap,
}

impl<M> Balloon<M>
where
    M: GuestAddressSpace + Clone + Send + 'static,
{

    pub fn new<B>(env: &mut Env<M, B>, args: &BalloonArgs) -> Result<Arc<Mutex<Self>>>
    where
        // We're using this (more convoluted) bound so we can pass both references and smart
        // pointers such as mutex guards here.
        B: DerefMut,
        B::Target: MmioManager<D = Arc<dyn DeviceMmio + Send + Sync>>,
    {
        let device_features = (1 << VIRTIO_F_VERSION_1);


        let queues = vec![
            Queue::new(env.mem.clone(), QUEUE_MAX_SIZE),
            Queue::new(env.mem.clone(), QUEUE_MAX_SIZE),
        ];


        let mut config_space = Vec::new();
        config_space.push(0);//virtioballoon_config
        config_space.push(0);
        config_space.push(0);
        config_space.push(0);
        let virtio_cfg = VirtioConfig::new(device_features, queues, config_space);

        let common_cfg = CommonConfig::new(virtio_cfg, env).map_err(Error::Virtio)?;

        let balloon = Arc::new(Mutex::new(Balloon {
            cfg: common_cfg,
            guest_memory: args.guest_memory.clone()

        }));

        // Register the device on the MMIO bus.
        env.register_mmio_device(balloon.clone())
            .map_err(Error::Virtio)?;

        Ok(balloon)
    }
    pub fn change_config(&mut self,size: u64){
        self.write(256, &size.to_le_bytes());
        self.cfg.virtio.interrupt_status.fetch_or(VIRTIO_MMIO_INT_CONFIG, Ordering::SeqCst);
        self.cfg.irqfd.write(1).expect("fail write to eventfd");
    }
}

impl<M: GuestAddressSpace + Clone + Send + 'static> Borrow<VirtioConfig<M>> for Balloon<M> {
    fn borrow(&self) -> &VirtioConfig<M> {
        &self.cfg.virtio
    }
}

impl<M: GuestAddressSpace + Clone + Send + 'static> BorrowMut<VirtioConfig<M>> for Balloon<M> {
    fn borrow_mut(&mut self) -> &mut VirtioConfig<M> {
        &mut self.cfg.virtio
    }
}

impl<M: GuestAddressSpace + Clone + Send + 'static> VirtioDeviceType for Balloon<M> {
    fn device_type(&self) -> u32 {
        BALLOON_DEVICE_ID
    }
}

impl<M: GuestAddressSpace + Clone + Send + 'static> VirtioDeviceActions for Balloon<M> {
    type E = Error;

    fn activate(&mut self) -> Result<()> {
        let driver_notify = SingleFdSignalQueue {
            irqfd: self.cfg.irqfd.clone(),
            interrupt_status: self.cfg.virtio.interrupt_status.clone(),
        };

        let mut ioevents = self.cfg.prepare_activate().map_err(Error::Virtio)?;

        let inner = SimpleHandler{
            driver_notify,
            inflate: self.cfg.virtio.queues.remove(0),
            deflate: self.cfg.virtio.queues.remove(0),
            guest_mem: self.guest_memory.clone(),
            inflate_page_num: 0
        };

        let handler = Arc::new(Mutex::new(QueueHandler {
            inner,
            inflate_io: ioevents.remove(0),
            deflate_io: ioevents.remove(0),
        }));

        self.cfg.finalize_activate(handler).map_err(Error::Virtio)
    }

    fn reset(&mut self) -> Result<()> {
        // Not implemented for now.
        Ok(())
    }
}

impl<M: GuestAddressSpace + Clone + Send + 'static> VirtioMmioDevice<M> for Balloon<M> {}

impl<M: GuestAddressSpace + Clone + Send + 'static> MutDeviceMmio for Balloon<M> {
    fn mmio_read(&mut self, _base: MmioAddress, offset: u64, data: &mut [u8]) {
        self.read(offset, data);
    }

    fn mmio_write(&mut self, _base: MmioAddress, offset: u64, data: &[u8]) {
        self.write(offset, data);
    }
}

