.PHONY: all build clean clean-img run

all: yoyo.img run

clean:
	cargo clean

clean-img:
	rm yoyo.img boot-part.img

build:
	cargo b

yoyo.img: build
	dd if=/dev/zero of=yoyo.img bs=512 count=93750
	parted yoyo.img -s -a minimal mklabel gpt
	parted yoyo.img -s -a minimal mkpart EFI FAT16 2048s 93716s
	parted yoyo.img -s -a minimal toggle 1 boot
	dd if=/dev/zero of=boot-part.img bs=512 count=91669
	mformat -i boot-part.img -h 32 -t 32 -n 64 -c 1
	mmd -i boot-part.img ::efi
	mmd -i boot-part.img /efi/boot
	mcopy -D o -i boot-part.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/efi/boot/BOOTx64.efi
	dd if=boot-part.img of=yoyo.img bs=512 count=91669 seek=2048 conv=notrunc

run:
	qemu-system-x86_64 -bios OVMF_CODE.fd -enable-kvm -cpu qemu64 -hda yoyo.img
