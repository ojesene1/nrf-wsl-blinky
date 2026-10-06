# nrf54l15-blinky

Blinky for the **nRF54L15 DK** written in Rust with [Embassy](https://embassy.dev), built and flashed from
**WSL 2** using [probe-rs](https://probe.rs). LED0 (pin `P2.09`) toggles once a second.

This README records everything needed to get from a fresh Windows + WSL machine to a blinking LED,
including the problems we hit along the way.

## 1. Prerequisites

### Windows
- Windows 10/11 with **WSL 2** (check with `wsl -l -v` in PowerShell; the VERSION column must say `2`.
  Convert with `wsl --set-version <Distro> 2` if needed - WSL 1 cannot do USB).
- [usbipd-win](https://github.com/dorssel/usbipd-win): `winget install usbipd`

### Inside WSL
- Rust (via [rustup](https://rustup.rs)) and the Cortex-M33 target:
  ```bash
  rustup target add thumbv8m.main-none-eabihf
  ```
- probe-rs (the **Linux** build - a Windows install of probe-rs is not visible from WSL):
  ```bash
  curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
  source ~/.cargo/env
  probe-rs --version
  ```
- `usbutils` for checking USB: `sudo apt install -y usbutils`

## 2. Connect the board to WSL

WSL cannot see USB devices on its own; the DK's on-board J-Link debugger has to be handed over from Windows.

1. Plug the DK into the PC (the USB port on the debugger side) and keep a WSL terminal open.
2. In an **admin PowerShell**:
   ```powershell
   usbipd list                                   # find the SEGGER J-Link line (VID:PID 1366:xxxx), note its BUSID e.g. 2-3
   usbipd bind --busid 2-3                       # one-time
   usbipd attach --wsl --busid 2-3 --auto-attach # re-attaches after unplug while this window stays open
   ```
3. In WSL, check it arrived:
   ```bash
   lsusb          # should list a SEGGER J-Link
   probe-rs list  # should list the probe
   ```

### udev rules (so your Linux user may open the probe)
```bash
sudo curl -L https://probe.rs/files/69-probe-rs.rules -o /etc/udev/rules.d/69-probe-rs.rules
sudo udevadm control --reload && sudo udevadm trigger
sudo usermod -aG plugdev $USER
```
If `udevadm` says udev isn't running, enable systemd by adding this to `/etc/wsl.conf`:
```
[boot]
systemd=true
```
then run `wsl --shutdown` in PowerShell, reopen WSL and re-run the `usbipd attach` command.

## 3. Build and flash

```bash
cargo run --release
```
`.cargo/config.toml` sets the runner to `probe-rs run --chip nRF54L15`, so `cargo run` builds, flashes and
stays attached to print log output. Press **Ctrl+C** to detach - the firmware keeps running on the board.

For `defmt` logs, set a log level (otherwise nothing is printed):
```bash
DEFMT_LOG=info cargo run --release
```
To make that permanent, add to `.cargo/config.toml`:
```toml
[env]
DEFMT_LOG = "info"
```
`info!` lines in `src/main.rs` are currently commented out; uncomment them (and `use defmt::info;`) to see logs.

If Ctrl+C prints no backtrace, use `probe-rs run --chip nRF54L15 --always-print-stacktrace <elf>`.

## 4. Project notes

| File | Purpose |
|---|---|
| `.cargo/config.toml` | Target `thumbv8m.main-none-eabihf`, linker args, probe-rs runner |
| `memory.x` | Flash/RAM layout for the nRF54L15 application core |
| `Cargo.toml` | Embassy from git (nRF54L15 support is not in the crates.io release), defmt 1.x |
| `src/main.rs` | Blinky on `P2_09` (LED0) |

`main.rs` clears leftover GRTC interrupt state before `embassy_nrf::init` (see Troubleshooting).

## 5. Troubleshooting log (what went wrong and why)

| Symptom | Cause | Fix |
|---|---|---|
| `could not execute process probe-rs ... Permission denied (os error 13)` | probe-rs not installed in WSL (the error is misleading - PATH contained unreadable Windows folders) | Install the Linux probe-rs (section 1) |
| `/sys/bus/usb/devices/ not found`, `No connected probes were found` | WSL has no USB devices until one is attached | usbipd bind/attach (section 2); needs WSL 2 |
| `The chip 'nRF54L15_xxAA' was not found in the database` | probe-rs names the chip `nRF54L15` | Use `--chip nRF54L15` (`probe-rs chip list \| grep -i 54l` to check) |
| Chip locked / protected on flash | Previously programmed or protected device | `probe-rs erase --chip nRF54L15 --allow-erase-all` once |
| HardFault / "Undefined instruction" right after boot | Panic (panic-probe halts via an invalid instruction), logs hidden because `DEFMT_LOG` unset; also mixed defmt 0.3 / 1.x versions | Align `defmt` 1.0.1, `defmt-rtt` 1.0.0, `panic-probe` 1.0.0 as in Embassy's nRF54L15 example; run with `DEFMT_LOG=info` |
| LED never blinks | `P1_04` is the DK's UART TX, not an LED | LED0 is `P2_09` |
| `No space left on device` / `Input/output error` while building | C: drive full (project lived under `/mnt/c`, inside OneDrive) | Free space, `cargo clean`; better, keep the project in the WSL home directory (`~/`) outside OneDrive |
| Flash succeeds, no log output, never reaches `main` code; backtrace in `RtcDriver::on_interrupt` (`GRTC_2`) | GRTC lives in an always-on domain, so interrupt state from the DK's previous (factory) firmware survives reset and floods the Embassy handler | `reset_grtc_leftovers()` in `main.rs` runs before `embassy_nrf::init`. A USB power cycle of the DK also clears it |

## 6. Tips
- `probe-rs run` never exits by itself - it keeps streaming logs until Ctrl+C. That is expected.
- The fast-blinking LED near the USB connector is the debugger's status LED, not LED0.
- Building inside `/mnt/c` works but is slow; copy the project into `~/` for faster builds.
