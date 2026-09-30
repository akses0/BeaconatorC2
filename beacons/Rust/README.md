# BeaconatorC2 - Rust Beacon

This is a beacon for BeaconatorC2, implemented in Rust.

## Design goals

### Flexible Transport architecture

BeaconatorC2 supports a flexible transport architecture, allowing for TCP, UDP, HTTP and SMB receivers. 

The rust beacon will implement these transports in the following order:

1. HTTP
2. Raw TCP
3. Raw UDP
4. SMB

### Encoding strategies

Similarly, BeaconatorC2 supports a flexible encoding strategy, allowing for for Base64, XOR and ROT13 encoding. The rust beacon will support these encoding strategies in the following order:

1. Base64
2. XOR
3. ROT13

### Stable keylogging

My intention is to explore stable keylogging. My development environment is currently linux, but I have access to windows and mac machines, so I can test the beacon on these platforms too. I will probably test this out on linux first, and then move to windows and finally macintosh systems. If I am feeling brave I have iPads and iPhones too but lets not get ahead of ourselves.
