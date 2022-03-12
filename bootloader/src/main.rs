#![no_main]
#![no_std]
#![feature(abi_efiapi)]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::string::String;
use uefi::prelude::*;

#[entry]
fn efi_main(_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap_success();

    // output firmware-vendor (CStr16 to Rust string)
    let mut buf = String::new();
    system_table.firmware_vendor().as_str_in_buf(&mut buf).unwrap();

    info!("Firmware Vendor: {}", buf.as_str());

    Status::SUCCESS
}
