# Reliable Serial (COMP.CE.340 Project Work 2)

A reliable serial server application for ESP32-C3 DevKit Rust 1. Provides functionality such as

- a blinking led
- a color changing RGB led
- a counter

supported via a reliable serial protocol.

## Flashing and running programs and examples

ESP32-C3 programs can be run on the target device as follows.

```sh
# Run an example from the examples directory
cargo embed --example smartled --release

# Run the program, defined at src/main.rs
cargo embed --release
```

## Running the reference example

You may use the following command to flash the reference example onto the board:

```sh
# Run the reference example (replace version number if necessary)
cargo flash --release --path pw2p1-reference-2025.r1.elf --chip esp32c3 --probe 303a:1001
```

You will get a superfluous warning as follows, but it does not impact the behavior:

```txt
WARN probe_rs::session: Failed to deconfigure device during shutdown: A RISC-V specific error occurred.

Caused by:
    Error occurred during execution of an abstract command: HaltResume
```

Note that the RTT terminal is not attached automatically and you will not see any output. You will
only be able to interact with the device by running the examples in
[../tester](../tester/examples/). Note also that there is a minor error with the UART driver leaking
one bit in the beginning, which sometimes causes the first transmission from host to fail. If this
happens, try again.

## Hardware setup

The system uses the following pins to provide the serial server:

| ESP32-C3  | Host  |
| -:        | :-    |
| IO21/TX   | RX    |
| IO20/RX   | TX    |

We recommend the use of an FT2232H Mini Module to allow monitoring the serial pins over a USB port
on a desktop host. Instructions for setting up the FT2232H Mini Module should be available on
Moodle.

## Compile & run

Use `cargo embed --release` to build & run the project on connected hardware. `cargo embed` searches
for `Embed.toml` in the invocation directory, so make sure to run `cargo embed` from the correct
directory. The default implementation provides some diagnostics over the serial Real-Time Transfer
(RTT) channel which is opened automatically by the provided `Embed.toml` file used by `cargo embed`.
