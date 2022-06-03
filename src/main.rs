#![no_main]
#![no_std]
#![feature(abi_efiapi)]

extern crate alloc;

use alloc::string::ToString;
use alloc::{string::String, vec::Vec};
use core::fmt::Write;
use log::{error, info};
use uefi::prelude::*;
use uefi::proto::console;

const EFI_PAGE_SIZE: u64 = 0x1000;

#[entry]
fn main(_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    // TODO: allegedly if watchdog timer is not disabled
    // system will reboot after 5 minutes, did not happen in tests.
    // system_table
    //     .boot_services()
    //     .set_watchdog_timer(0, 65536, None)
    //     .unwrap();

    system_table.stdout().reset(false).unwrap();
    system_table.stdout().clear().unwrap();

    loop {
        write!(system_table.stdout(), "> ").unwrap();
        let line = read_line(&mut system_table);
        if line == "exit" {
            break;
        }
        run_command(&mut system_table, line);
    }
    Status::SUCCESS
}

/// Blocks until key is pressed then returns the key
fn wait_for_key(system_table: &mut SystemTable<Boot>) -> console::text::Key {
    loop {
        // Safety: the Event is not used in other places so it can't be invalidated
        // while waiting for it here
        let event = unsafe { system_table.stdin().wait_for_key_event().unsafe_clone() };
        let mut event_list = [event];

        system_table
            .boot_services()
            .wait_for_event(&mut event_list)
            .unwrap();

        if let Some(key) = system_table.stdin().read_key().unwrap() {
            return key;
        }
    }
}

fn read_line(system_table: &mut SystemTable<Boot>) -> String {
    let mut ret = String::new();

    loop {
        let key = wait_for_key(system_table);
        if let console::text::Key::Printable(key) = key {
            // Break on Enter pressed
            if key == uefi::Char16::try_from('\r').unwrap() {
                writeln!(system_table.stdout()).unwrap();
                break;
            }

            // On Backspace, delete character from string
            if key == uefi::Char16::try_from('\u{8}').unwrap() {
                if ret.is_empty() {
                    continue;
                }

                ret.pop();
            }

            let character: char = key.into();
            write!(system_table.stdout(), "{}", character).unwrap();

            if character.is_alphanumeric() {
                ret.push(character);
            }
        }
    }

    ret
}

fn run_command(system_table: &mut SystemTable<Boot>, command: String) {
    let arguments = command
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    if arguments.is_empty() {
        error!("No command given!");
        return;
    }

    match arguments[0].as_str() {
        "version" => print_version(system_table),
        "memorymap" => print_memory_map(system_table),
        "echo" => echo(system_table.stdout(), &arguments[1..]),
        _ => {
            error!("Command \"{}\" not found!", arguments[0]);
        }
    }
}

fn echo(stdout: &mut console::text::Output, arguments: &[String]) {
    for word in arguments {
        write!(stdout, "{}", word).unwrap();
        write!(stdout, " ").unwrap();
    }
    writeln!(stdout).unwrap();
}

fn print_version(system_table: &mut SystemTable<Boot>) {
    let mut text = String::new();

    system_table
        .firmware_vendor()
        .as_str_in_buf(&mut text)
        .unwrap();

    let uefi_rev = system_table.uefi_revision();

    writeln!(
        system_table.stdout(),
        "Vendor: {}, UEFI {}.{}",
        text,
        uefi_rev.major(),
        uefi_rev.minor()
    )
    .unwrap();
}

fn print_memory_map(system_table: &mut SystemTable<Boot>) {
    let bs = system_table.boot_services();
    let map_size = bs.memory_map_size();

    let mut buffer = alloc::vec![0; map_size.map_size + 64];

    let (_k, desc_iter) = bs.memory_map(&mut buffer).unwrap();

    let descriptors = desc_iter.copied().collect::<Vec<_>>();

    assert!(!descriptors.is_empty(), "Memory map is empty");

    writeln!(
        system_table.stdout(),
        "UEFI: usable memory ranges ({} total)",
        descriptors.len()
    )
    .unwrap();

    for desc in descriptors {
        if desc.ty == uefi::table::boot::MemoryType::CONVENTIONAL {
            let size = desc.page_count * EFI_PAGE_SIZE;
            let end_addr = desc.phys_start + size;

            writeln!(
                system_table.stdout(),
                " {:?} {:#x} - {:#x} ({} KiB)",
                desc.ty,
                desc.phys_start,
                end_addr,
                size
            )
            .unwrap();
        }
    }
}
