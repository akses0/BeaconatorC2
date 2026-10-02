# BeaconatorC2 - Rust Beacon

This is a beacon for BeaconatorC2, implemented in Rust.

## Design goals

### Flexible Transport architecture

BeaconatorC2 supports a flexible transport architecture, allowing for TCP, UDP, HTTP and SMB receivers. 

The rust beacon will implement these transports in the following order:

1. Raw TCP - done
2. HTTP
3. Raw UDP
4. SMB

### Obfuscation strategies

The following obfuscation strats are implemented

1. Base64
2. XOR
3. ROT13
4. Plaintext

### Stable keylogging

My intention is to explore stable keylogging. My development environment is currently linux, but I have access to windows and mac machines, so I can test the beacon on these platforms too. I will probably test this out on linux first, and then move to windows and finally macintosh systems. If I am feeling brave I have iPads and iPhones too but lets not get ahead of ourselves.
