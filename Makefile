compile:  
	@make --no-print-directory -C bootloader
	@make --no-print-directory -C kernel

all: compile

run: iso
	$(shell bash ./run.sh)

iso: compile
	@rm -rf dist
	@mkdir -p dist/EFI/Boot/
	cp bootloader/target/x86_64-unknown-uefi/debug/bootloader.efi dist/EFI/Boot/boot.efi
	cp bootloader/startup.nsh dist/
	cp bootloader/config.conf dist/EFI/Boot/
	cp kernel/target/x86_64-unknown-linux-gnu/debug/kernel dist/kernel.efi
	dd if=/dev/zero of=Mulberry.img bs=1M count=100
	mformat -Fi Mulberry.img ::
	mcopy -si Mulberry.img dist/* ::

fix:
	@rm -rf OVMFbin
	git clone https://github.com/platinumTypeC/OVMFbin.git OVMFbin

clean:
	@make clean --no-print-directory -C bootloader
	@make clean --no-print-directory -C kernel
