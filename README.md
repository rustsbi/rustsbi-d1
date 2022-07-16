# 哪吒引导工具

哪吒引导和引导程序调试工具。

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

## 加载过程

支持以下模式：

1. xfel -> spl -> see -> kernel
2. xfel --------> see -> kernel
3. brom -> spl -> see -> kernel

每种模式都支持在没有后续环节时停住。

## 命令

环境参数：

- `--spl` 调试 spl
- `--see` 调试 see
- `--kernel <file>` 过程包含指定的特权程序二进制文件
- `--dt <file>` 过程包含指定的设备树文件

命令：

- **`cargo make`**
- **`cargo asm`**
- **`cargo debug`**
- **`cargo flash`**
