# Connecting to the device serial port via USB (FTDI chip required)

You cannot put serial wires into a USB port and expect it to work. Therefore we must use a small
FTDI2232HL board to provide protocol translation.

The FTDI's pins should be configured the following way:

## Internal (loopback) connections

| **Start** | **End** | **Description** |
| :-:       | :-:     | :-:             |
| CN3-1     | CN3-3   | Connect the USB power to the VCC of the FTDI |
| CN2-3     | CN2-11  | Connect the VCC of the FTDI to the VCC IO of the FTDI |

## External connections to the ESP32-C3

| FT2232H Mini  | ESP32-C3 |
| :-:           | :-:      |
| CN2-7 (TX)    | IO20/RX  |
| CN2-10 (RX)   | IO21/TX  |

## Sources

- [https://ftdichip.com/wp-content/uploads/2020/07/DS_FT2232H_Mini_Module.pdf](https://ftdichip.com/wp-content/uploads/2020/07/DS_FT2232H_Mini_Module.pdf)
- [https://ftdichip.com/wp-content/uploads/2020/07/DS_FT2232H.pdf](https://ftdichip.com/wp-content/uploads/2020/07/DS_FT2232H.pdf)

## Connecting to the FTDI

In order to connect to the FTDI and interact with the ESP32-C3 via serial, we will use PuTTY. For
the course VM, just run `sudo putty` and select the `FTDI Serial` profile (**for those using the
couse VM**). Click open and now, if you have connected the FTDI to the VM, you should see the data
the ESP32-C3 sends over the serial!
