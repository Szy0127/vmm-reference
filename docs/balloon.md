# balloon

## 设计与改动

1. 增加balloon device config 通过`--balloon xx` (xx无作用)给vmm加入balloon设备
2. 增加balloon virtio device
   -  mmio、irq、virtio queue共享内存通信，依赖vm-virtio实现
   - 初始化好deviceID、config、feature、队列
   - 给Balloon设备加上GuestMemoryMmap的mem 便于get_host_address拿到hva
   - inflate和deflate通过madvise接口实现
3. 修改主循环，另启线程监听balloon请求与关闭虚拟机请求
   - 用Arc<Mutex<Vmm>>共享vmm对象
   - 将主循环需要用到的event_manager和exit_handler提取出vmm对象

## 运行与测试

guest kernel config：CONFIG_VIRTIO_BALLOON=y

启动：

`./target/debug/vmm-reference --memory size_mib=4096 --vcpu num=2 --kernel path=./ubuntu-focal/linux-5.4.81/arch/x86/boot/bzImage --block path=/tmp/ubuntu-focal/rootfs.ext4 --net tap=vmtap100 --balloon 0`

通过/tmp/rust-vmm.sock 进行通信

inflate/deflate:

`./scripts/balloon.py 1024` inflate, reclaim 1G

`./scripts/balloon.py 0` deflate, give back 1G

guest/host可通过free -mh 或numactl -H 观察内存变化

guest内可以使用memhog申请内存