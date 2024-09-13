﻿# RustSBI implementation for Allwinner D1

[![CI](https://github.com/rustsbi/rustsbi-d1/actions/workflows/workflow.yml/badge.svg?branch=main)](https://github.com/rustsbi/rustsbi-d1/actions)
[![issue](https://img.shields.io/github/issues/rustsbi/rustsbi-d1)](https://github.com/rustsbi/rustsbi-d1/issues)

全志 D1 引导程序及其调试工具。

## 模块

### SPL

运行在 SRAM，单独调试时产生如下输出：

```text
DRAM only have internal ZQ!!
get_pmu_exist() = 4294967295
ddr_efuse_type: 0x0
[AUTO DEBUG] two rank and full DQ!
ddr_efuse_type: 0x0
[AUTO DEBUG] rank 0 row = 16
[AUTO DEBUG] rank 0 bank = 8
[AUTO DEBUG] rank 0 page size = 2 KB
[AUTO DEBUG] rank 1 row = 16
[AUTO DEBUG] rank 1 bank = 8
[AUTO DEBUG] rank 1 page size = 2 KB
rank1 config same as rank0
DRAM BOOT DRIVE INFO: %s
DRAM CLK = 792 MHz
DRAM Type = 3 (2:DDR2,3:DDR3)
DRAMC ZQ value: 0x7b7bfb
DRAM ODT value: 0x42.
ddr_efuse_type: 0x0
DRAM SIZE =2048 M
DRAM simple test OK.

   _  __        __          ___            __    __  ____  _ __
  / |/ /__ ___ / /  ___ _  / _ )___  ___  / /_  / / / / /_(_) /
 /    / -_)_ // _ \/ _ `/ / _  / _ \/ _ \/ __/ / /_/ / __/ / /
/_/|_/\__//__/_//_/\_,_/ /____/\___/\___/\__/  \____/\__/_/_/🦀
no payload |                     <<                           |
```

### SEE

运行在 DRAM，单独调试时产生如下输出：

```text
[rustsbi] no dtb file detected
[rustsbi] RustSBI version 0.3.0-alpha.1, adapting to RISC-V SBI v1.0.0
.______       __    __      _______.___________.  _______..______   __
|   _  \     |  |  |  |    /       |           | /       ||   _  \ |  |
|  |_)  |    |  |  |  |   |   (----`---|  |----`|   (----`|  |_)  ||  |
|      /     |  |  |  |    \   \       |  |      \   \    |   _  < |  |
|  |\  \----.|  `--'  |.----)   |      |  |  .----)   |   |  |_)  ||  |
| _| `._____| \______/ |_______/       |__|  |_______/    |______/ |__|
[rustsbi] Implementation     : RustSBI-D1 Version 0.1.0
[rustsbi] Extensions         : [legacy console, timer, reset, ipi]
[rustsbi] Platform Name      : unknown
[rustsbi] Platform SMP       : 1
[rustsbi] Platform Memory    : 0x0..0x0
[rustsbi] Boot HART          : 0
[rustsbi] Device Tree Region : 0x0..0x0
[rustsbi] Firmware Address   : 0x40000000
[rustsbi] Supervisor Address : 0x0
[rustsbi] no kernel |                                      <<         |
```

### TEST-KERNEL

用于测试 SEE 的 Supervisor，若 SEE 工作正常，产生如下输出：

```text
 _____         _     _  __                    _
|_   _|__  ___| |_  | |/ /___ _ __ _ __   ___| |
  | |/ _ \/ __| __| | ' // _ \ '__| '_ \ / _ \ |
  | |  __/\__ \ |_  | . \  __/ |  | | | |  __/ |
  |_|\___||___/\__| |_|\_\___|_|  |_| |_|\___|_|
================================================
| boot hart id          |                    0 |
| smp                   |                    1 |
| timebase frequency    |          24000000 Hz |
| dtb physical address  |           0x7fe00000 |
------------------------------------------------
[ INFO] Testing Base
[ INFO] sbi spec version = 1.0
[ INFO] sbi impl = RustSBI
[ INFO] sbi impl version = 0x300
[ INFO] sbi extensions = [Base, TIME, sPI, SRST]
[ INFO] mvendor id = 0x5b7
[ INFO] march id = 0x0
[ INFO] mimp id = 0x0
[ INFO] Sbi Base Test Pass
[ INFO] Testing TIME
[ INFO] read time register successfuly, set timer +1s
[ INFO] timer interrupt delegate successfuly
[ INFO] Sbi TIME Test Pass
[ INFO] Testing sPI
[ INFO] send ipi successfuly
[ INFO] Sbi sPI Test Pass
[ERROR] Sbi HSM Not Exist
[ INFO] marchid duration = 13468308
[ INFO] ipi duration = 28632525
[rustsbi] system reset |                   >>  |
```

## 加载过程

支持以下模式：

1. xfel -> spl -> see -> kernel

   > 这个模式目前不能工作，因为一旦使用 `xfel ddr d1`，就没法从 sram 运行了，原因不明

2. xfel --------> see -> kernel

3. brom -> spl -> see -> kernel

每种模式都支持在没有后续环节时停住。

## 命令

环境参数：

- **`--spl`**：命令指定的操作将加载 spl。
- **`--see`**：命令指定的操作将加载 see。
- **`--kernel <file/::test>`**：命令指定的操作将加载指定内核文件或测试内核。
- **`--dt <file>`**：命令指定的操作将加载指定设备树文件。

命令：

- **`cargo make`**

  生成各阶段目标文件。

  示例：

  - `cargo make --spl` 生成 spl.bin
  - `cargo make --spl --see` 生成 spl.bin 和 see.bin
  - `cargo make --spl --see --dt nezha.dts` 生成 spl.bin、see.bin 和 nezha.dtb

- **`cargo asm`**

  生成各阶段反汇编文件。

  参数：

  - `-o`/`--output` 文件保存位置，默认保存到 target 目录下。

  示例：

  - `cargo asm` 生成 `target/spl.asm` 和 `target/see.asm`
  - `cargo asm --see -o sbi.asm` 在当前目录生成 `sbi.asm`

  > **NOTICE**
  >
  > - `cargo asm` 视作 `cargo asm --spl --see`
  > - `--kernel` 和 `--dt` 目前无效

- **`cargo debug`**

  调试，不使用 flash。

  示例：

  - `cargo debug --spl` 调试 spl
  - `cargo debug --see` 调试 see
  - `cargo debug --see --dt nezha.dts` 调试可见设备树文件的 see
  - `cargo debug --see --kernel zcore.bin --dt nezha.dts` 调试 see + kernel

- **`cargo flash`**

  烧写到 flash。

  环境参数的 4 块对于此命令是独立的。

  参数：

  - `--reset` 重置元数据，即格式化 flash
  - `--boot` 此次烧写完成后从 brom 重启

  示例：

  - `cargo flash --spl --boot` 烧写 spl，完成后立即重启
  - `cargo flash --see --reset` 烧写 see，并格式化 flash，丢弃以前的 kernel 和 dtb
  - `cargo flash --dt nezha.dts` 烧写设备树
  - `cargo flash --kernel zcore.bin` 烧写内核
  - `cargo flash --boot` 立即从 brom 重启

## 换行问题

如果你使用 minicom 连接开发板，出现显示时光标不回行首的情况（类似[这样](https://github.com/rustsbi/rustsbi-d1/issues/1)），需要改 minicom 配置，参考[此问答](https://unix.stackexchange.com/questions/283924/how-can-minicom-permanently-translate-incoming-newline-n-to-crlf)。

## 错误提示 ERROR xtask::xfel] xfel not found

xfel: https://github.com/xboot/xfel

Windows配置: 下载[`xfel-windows-v1.3.2.7z`](https://github.com/xboot/xfel/releases/download/v1.3.2/xfel-windows-v1.3.2.7z)后解压，并将解压目录路径添加至系统环境变量 Path 中，于重新打开的终端中运行`xfel`命令验证成功。
