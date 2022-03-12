compile:
	@make --no-print-directory -C bootloader

all: compile

runwsl: iso
	qemu-system-x86_64.exe -drive file=Mulberry.img,format=raw -m 200M -cpu qemu64 -drive if=pflash,format=raw,unit=0,file="OVMFbin/OVMF_CODE-pure-efi.fd",readonly=on -drive if=pflash,format=raw,unit=1,file="OVMFbin/OVMF_VARS-pure-efi.fd";

runlinux: iso
	qemu-system-x86_64 -drive file=Mulberry.img,format=raw -m 200M -cpu qemu64 -drive if=pflash,format=raw,unit=0,file="OVMFbin/OVMF_CODE-pure-efi.fd",readonly=on -drive if=pflash,format=raw,unit=1,file="OVMFbin/OVMF_VARS-pure-efi.fd";

iso: compile
	@rm -rf dist
	@mkdir -p dist/EFI/Boot/
	cp bootloader/target/x86_64-unknown-uefi/debug/bootloader.efi dist/EFI/Boot/boot.efi
	cp bootloader/startup.nsh dist/
	dd if=/dev/zero of=Mulberry.img bs=1M count=100
	mformat -Fi Mulberry.img ::
	mcopy -si Mulberry.img dist/* ::

fix:
	@rm -rf OVMFbin
	git clone https://github.com/platinumTypeC/OVMFbin.git OVMFbin
