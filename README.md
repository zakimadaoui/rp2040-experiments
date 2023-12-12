# RP-PICO/rp2040 Rust Experiments Workspace

**Work in Progress...**. Feel free to explore and experiment with different Rust configurations for RP-PICO/rp2040 in this workspace.

## Usage

### Install Prerequisits
``` bash
rustup target install thumbv6m-none-eabi
cargo install elf2uf2-rs --locked
```

### Build

To build the project, you can use the following commands:

```bash
# Build all crates
cargo build --all

# Or build a specific crate
cargo build --bin <crate_name>
```

### Flashing Firmware to RP-PICO

Follow these steps to flash firmware onto the RP-PICO:
1) Connect a micro USB cable to the RP-PICO.
2) While pressing the BOOTSEL button, attach the other end of the USB cable to your PC.
3) Use the following command to flash and run the desired firmware:

```bash
cargo run --bin <crate_name>
```
the .cargo/config.toml already configures the target (thumbv6m-none-eabi) to compile for the RP2040 and configures the runner (elf2uf2-rs in this case)

4) To flash another firmware, repeat steps 2-3.