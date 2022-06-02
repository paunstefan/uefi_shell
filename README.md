# UEFI_shell

Simple UEFI application made for learning purposes.

To build and run:

```sh
rustup toolchain install nightly
rustup component add --toolchain nightly rust-src
rustup override set nightly

uefi-run -b /path/to/OVMF.fd -q /path/to/qemu app.efi -- <extra_qemu_args>
```
