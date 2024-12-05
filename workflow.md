# Download, build, and run everything
## Tested on Ubuntu 24.04
## Prepare the directories, assuming you don't already have a `work` directory in your home
```
mkdir ~/work && cd ~/work
```
## Get the vanilla Linux kernel, just the top of a single branch to make it quicker
```
git clone --single-branch --branch=v6.12 --depth=1 https://github.com/torvalds/linux.git
```
## Setup the Rust programming language environment
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```
## Add Rust components required for building Linux kernel modules
```
rustup component add rust-src
cargo install --locked bindgen-cli
```
## Install the stuff for the build and the demo (probably not a comprehensive list)
```
sudo apt update && sudo apt upgrade
sudo apt install git fakeroot kpartx build-essential ncurses-dev xz-utils libssl-dev bc flex bison libelf-dev libncurses-dev openssl libssl-dev dkms libudev-dev libpci-dev pulseaudio pulseaudio-utils libiberty-dev autoconf qemu-system clang llvm lld libclang-dev gdb ccache
```
## Setup Linux kernel directory for a build with Rust using the current Linux configuration
```
cd linux/
export CC="ccache clang" && export CXX="ccache clang++"
make LLVM=1 rustavailable # Just a sanity check, should report OK
cp /boot/config-$(uname -r) .config
./scripts/config -d CONFIG_DEBUG_INFO_BTF -d CONFIG_MODVERSIONS -e CONFIG_RUST -e CONFIG_SAMPLES_RUST -m CONFIG_SAMPLE_RUST_MINIMAL -m CONFIG_SAMPLE_RUST_LEON -e CONFIG_SAMPLES -d CONFIG_SYSTEM_TRUSTED_KEYS -d CONFIG_SYSTEM_REVOCATION_LIST -e CONFIG_NET_9P -e CONFIG_NET_9P_VIRTIO -e CONFIG_9P_FS -e CONFIG_9P_FS_POSIX_ACL -e CONFIG_9P_FS_SECURITY --set-val CONFIG_FRAME_WARN 2048 -e CONFIG_E1000
make LLVM=1 olddefconfig
make LLVM=1 rust-analyzer # This will set up the directory for using with code editors like VSCode
```
## Build the kernel (don't forget to have the CC env. variable set as stated above)
```
make LLVM=1 -j$(nproc) bzImage
make LLVM=1 -j$(nproc) modules
mkdir -p out/kernel-modules
make LLVM=1 modules_install INSTALL_MOD_PATH=out/kernel-modules
make LLVM=1 samples/rust/leon.ko # As needed, without rebuilding the rest
```
## Prepare the rootfs image
```
mkdir ../rootfs && cd ../rootfs
dd if=/dev/zero of=debian.img bs=1M count=2048
sudo losetup -fP debian.img
sudo losetup -a | grep -v snap # Check the loop device name. In my case - loop25.
sudo parted /dev/loop25
sudo kpartx -a /dev/loop25
sudo mkfs.ext4 /dev/mapper/loop25p1
mkdir mnt
sudo mount /dev/mapper/loop25p1 mnt
sudo apt install debootstrap
mkdir debian-rootfs
sudo debootstrap --arch=amd64 stable debian-rootfs http://deb.debian.org/debian/
sudo chroot debian-rootfs /bin/bash
```
### Inside chroot
```
mount -t proc /proc /proc
mount -t sysfs /sys /sys
mount -o bind /dev /dev
mount -o bind /dev/pts /dev/pts
apt update
apt install --no-install-recommends xserver-xorg xinit xterm x11-apps net-tools iproute2 iputils-ping ifupdown pulseaudio openssh-server vim
passwd -d root
```
```
cat <<EOF > /etc/network/interfaces
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF
```
```
echo "debian-vm" > /etc/hostname
echo "PermitRootLogin yes" >> /etc/ssh/sshd_config
echo "PermitEmptyPasswords yes" >> /etc/ssh/sshd_config
echo "X11Forwarding yes" >> /etc/ssh/sshd_config
echo "X11DisplayOffset 10" >> /etc/ssh/sshd_config
echo "X11UseLocalhost yes" >> /etc/ssh/sshd_config

apt clean
exit
```
### Back on the host
```
sudo umount debian-rootfs/proc
sudo umount debian-rootfs/sys
sudo umount debian-rootfs/dev/pts
sudo umount debian-rootfs/dev
sudo cp -a debian-rootfs/* mnt
sudo umount mnt
sudo kpartx -d /dev/loop25
sudo losetup -d /dev/loop25
```
## Make sure you can access the micro:bit device without root priveledges (verify your device vendor/product numbers with `lsusb`)
```
sudo echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", ATTR{idProduct}=="0204", MODE="0666", GROUP="plugdev"' >> /etc/udev/rules.d/50-microbit.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```
# Run it
## On the host
### This will build and flash the `micro:bit` code.
```
cd ~/work/micro_bit/rust-examples-microbit-2024/ex4_accelerometer && cargo embed && cd -
```
### Now let's start the VM
```
cd .. # You need to be in the work directory

qemu-system-x86_64 \
-enable-kvm \
-cpu host \
-m 8192 \
-smp 4 \
-kernel linux/arch/x86/boot/bzImage \
-hda rootfs/debian.img \
-append "root=/dev/sda1 rw console=ttyS0 net.ifnames=0" \
-serial stdio \
-netdev user,id=net0,hostfwd=tcp::2222-:22 \
-device e1000,netdev=net0 \
-virtfs local,path=linux/out/kernel-modules/lib/modules,mount_tag=modules,security_model=passthrough,id=modules \
-device intel-hda \
-device hda-duplex,audiodev=pa1 \
-audiodev pa,id=pa1 \
-machine pc \
-vga virtio \
-display gtk,gl=on \
-device qemu-xhci,id=xhci \
-device usb-host,vendorid=0x0d28,productid=0x0204
```
Note that we're mounting the kernel modules to the right spot inside the image, so that they are naturally in the right place. Now we first copy our module into the VM, and then ssh into it.
```
scp -P 2222 linux/samples/rust/rust_leon.ko root@localhost:~
ssh -X -p 2222 root@localhost
insmod rust_leon.ko
ldattach -s 115200 -8n1 29 /dev/ttyACM0
jstest-gtk
```
