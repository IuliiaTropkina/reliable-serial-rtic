# Sharing USB devices with Windows Subsystem for Linux (WSL)

Get [usbipd](https://github.com/dorssel/usbipd-win/).

In admin power shell:

1. `usbipd list`, identify bus id for your device
2. `usbipd bind --busid <busid>`, bind to the device (required first time only)
3. `usbipd attach --wsl --busid <busid>`, attach to WSL
    - If this returns `usbipd: error: Loading vhci_hcd failed.`, run `sudo modprobe vhci_hcd` in the WSL shell.

In WSL2:

`sudo udevadm control --reload-rules && sudo udevadm trigger`

On failure, run `sudo service udev restart` and try again.

If still fails, try loading the kernel module manually: `sudo modprobe vhci_hcd`
