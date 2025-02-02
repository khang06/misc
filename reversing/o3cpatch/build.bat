REM Too lazy to make a Makefile for this lol
SET TOOLCHAIN="C:\MounRiver\MRS_Community\toolchain\RISC-V Embedded GCC12"

%TOOLCHAIN%"/bin/riscv-none-elf-gcc.exe" -march=rv32imafcxw -mabi=ilp32f -O3 -nostartfiles -ffreestanding -Wl,-T,main.ld -Wl,-Map=main.map main.c -o main.o
%TOOLCHAIN%"/bin/riscv-none-elf-objcopy.exe" -O binary main.o main.bin
py -3.11 o3cpatch.py
