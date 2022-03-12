compile:
	@make --no-print-directory -C bootloader

all: compile

run: iso
	$(shell bash ./run.sh)

iso: compile
	@rm -rf dist
	@mkdir -p dist/EFI/Boot/
	cp bootloader/target/x86_64-unknown-uefi/debug/bootloader.efi dist/EFI/Boot/boot.efi
	cp bootloader/startup.nsh dist/
	dd if=/dev/zero of=Mapple.img bs=1M count=100
	mformat -Fi Mapple.img ::
	mcopy -si Mapple.img dist/* ::

fix:
	@rm -rf OVMFbin
	git clone https://github.com/platinumTypeC/OVMFbin.git OVMFbin
