# Real-time Transfer (RTT)

The examples contained within use the crates `rtt-target` and `panic-rtt-target` to provide printing
and panics based on a real-time transfer protocol. This is interlaced with the USB transfer, and
interpreted by probe-rs used via `cargo embed`.

RTT is used to provide such printing facilities that they minimally interfere with the timing of the
programs.
