# Cross-Core Signaling experiments
- `demo1.rs`: "Blocking PingPong example": Cross-core message exchange using FIFOs with Blocking approach 
- `demo2.rs`: Example of handling the same interrupt from both cores simultaniously 
- `demo3.rs`: Cross-core signaling 
- `demo4.rs`: "Non-Blocking PingPong example": Cross-core message exchange using FIFO interrupts as Proxy to forward signals.

### Usage

Experiments were conducted using 2 rp pico setups, one as a PICO PROBE, the other as DUT.

```bash 
cargo run --example demoN
```
