#![no_main]
#![no_std]
#![feature(abi_efiapi)]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use crate::types::GraphicInfo;
use uefi::prelude::*;
use uefi::proto::media::file::*;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::cfg::{ACPI2_GUID, SMBIOS_GUID};
use x86_64::registers::control::*;
use x86_64::structures::paging::*;
use x86_64::{PhysAddr, VirtAddr};
use xmas_elf::ElfFile;

mod types;
mod config;
mod page_table;

const CONFIG_PATH: &str = "\\EFI\\Boot\\config.conf";

struct UEFIFrameAllocator<'a>(&'a BootServices);


unsafe impl FrameAllocator<Size4KiB> for UEFIFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let addr = self
            .0
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .expect_success("failed to allocate frame");
        let frame = PhysFrame::containing_address(PhysAddr::new(addr));
        Some(frame)
    }
}

#[entry]
fn efi_main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect_success("failed to initialize utilities");

    // output firmware-vendor (CStr16 to Rust string)
    {
        let mut buf = String::new();
        system_table.firmware_vendor().as_str_in_buf(&mut buf).unwrap();
        info!("Firmware Vendor: {}", buf.as_str());
    }
    
    check_revision(system_table.uefi_revision());

    // Test all the boot services.
    let bs = system_table.boot_services();

    let config = {
        let mut file = open_file(bs, CONFIG_PATH);
        let buf = load_file(bs, &mut file);
        config::Config::parse(buf)
    };

    let graphic_info = init_graphic(bs, config.resolution);
    info!("config: {:#x?}", config);

    let acpi2_addr = system_table
        .config_table()
        .iter()
        .find(|entry| entry.guid == ACPI2_GUID)
        .expect("failed to find ACPI 2 RSDP")
        .address;

    info!("acpi2: {:?}", acpi2_addr);

    let smbios_addr = system_table
        .config_table()
        .iter()
        .find(|entry| entry.guid == SMBIOS_GUID)
        .expect("failed to find SMBIOS")
        .address;
    info!("smbios: {:?}", smbios_addr);

    let elf = {
        let mut file = open_file(bs, config.kernel_path);
        let buf = load_file(bs, &mut file);
        ElfFile::new(buf).expect("failed to parse ELF")
    };

    unsafe {
        ENTRY = elf.header.pt2.entry_point() as usize;
    }

    let (initramfs_addr, initramfs_size) = if let Some(path) = config.initramfs {
        let mut file = open_file(bs, path);
        let buf = load_file(bs, &mut file);
        (buf.as_ptr() as u64, buf.len() as u64)
    } else {
        (0, 0)
    };
 
    let max_mmap_size = system_table.boot_services().memory_map_size().map_size;
    let mmap_storage = Box::leak(vec![0; max_mmap_size * 2].into_boxed_slice());
    let mmap_iter = system_table
        .boot_services()
        .memory_map(mmap_storage)
        .expect_success("failed to get memory map")
        .1;
    let max_phys_addr = mmap_iter
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area
 
    let mut page_table = current_page_table();
    // root page table is readonly
    // disable write protect
    unsafe {
        Cr0::update(|f| f.remove(Cr0Flags::WRITE_PROTECT));
        Efer::update(|f| f.insert(EferFlags::NO_EXECUTE_ENABLE));
    }

    page_table::map_elf(&elf, &mut page_table, &mut UEFIFrameAllocator(bs))
        .expect("failed to map ELF");
    
    return Status::SUCCESS;
}

fn check_revision(rev: uefi::table::Revision) {
    let (major, minor) = (rev.major(), rev.minor());

    info!("UEFI {}.{}", major, minor / 10);

    assert!(major >= 2, "Running on an old, unsupported version of UEFI");
    assert!(
        minor >= 30,
        "Old version of UEFI 2, some features might not be available."
    );
}

fn open_file(bs: &BootServices, path: &str) -> RegularFile {
    info!("opening file: {}", path);
    // FIXME: use LoadedImageProtocol to get the FileSystem of this image
    let fs = bs
        .locate_protocol::<SimpleFileSystem>()
        .expect_success("failed to get FileSystem");
    let fs = unsafe { &mut *fs.get() };

    let mut root = fs.open_volume().expect_success("failed to open volume");
    let handle = root
        .open(path, FileMode::Read, FileAttribute::empty())
        .expect_success("failed to open file");

    match handle.into_type().expect_success("failed to into_type") {
        FileType::Regular(regular) => regular,
        _ => panic!("Invalid file type"),
    }
}

/// Load file to new allocated pages
fn load_file(bs: &BootServices, file: &mut RegularFile) -> &'static mut [u8] {
    info!("loading file to memory");
    let mut info_buf = [0u8; 0x100];
    let info = file
        .get_info::<FileInfo>(&mut info_buf)
        .expect_success("failed to get file info");
    let pages = info.file_size() as usize / 0x1000 + 1;
    let mem_start = bs
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .expect_success("failed to allocate pages");
    let buf = unsafe { core::slice::from_raw_parts_mut(mem_start as *mut u8, pages * 0x1000) };
    let len = file.read(buf).expect_success("failed to read file");
    &mut buf[..len]
}

fn init_graphic(bs: &BootServices, resolution: Option<(usize, usize)>) -> GraphicInfo {
    let gop = bs
        .locate_protocol::<GraphicsOutput>()
        .expect_success("failed to get GraphicsOutput");
    let gop = unsafe { &mut *gop.get() };

    if let Some(resolution) = resolution {
        let mode = gop
            .modes()
            .map(|mode| mode.expect("Warnings encountered while querying mode"))
            .find(|ref mode| {
                let info = mode.info();
                info.resolution() == resolution
            })
            .expect("graphic mode not found");
        info!("switching graphic mode");
        gop.set_mode(&mode)
            .expect_success("Failed to set graphics mode");
    }
    GraphicInfo {
        mode: gop.current_mode_info(),
        fb_addr: gop.frame_buffer().as_mut_ptr() as u64,
        fb_size: gop.frame_buffer().size() as u64,
    }
}

fn current_page_table() -> OffsetPageTable<'static> {
    let p4_table_addr = Cr3::read().0.start_address().as_u64();
    let p4_table = unsafe { &mut *(p4_table_addr as *mut PageTable) };
    unsafe { OffsetPageTable::new(p4_table, VirtAddr::new(0)) }
}

static mut ENTRY: usize = 0;
