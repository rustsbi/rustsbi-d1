# 哪吒引导工具

哪吒引导和引导程序调试工具。

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
NAND flash: c2 26 3
no payload[>>                                    ]
```

## 功能

- `cargo asm [--stage <sram/dram>] [--output <path>]`

  反汇编目标文件并保存到指定位置。

  - 若 `--stage` 为空，两个阶段都会生成
  - 若 `--output` 为空，保存在项目根目录

- `cargo boot [--stage <sram/dram>] [--kernel <path>] [--dtb <path>]`

  通过 xfel 直接引导。

  - 若 `--stage` 选择 `sram`，将整个引导流程、内核文件和 dtb 文件全部烧写到 flash，然后从 sram 启动
  - 若 `--stage` 选择 `dram`，将 see、kernel 和 dtb 文件放在 dram 合适的位置上，并从 dram 启动
  - 若 `--stage` 为空，默认按 `dram` 执行

- `cargo debug --stage <sram/dram>`

  调试引导程序。

- `cargo erase [--range <start..end/base[len]>]`

  擦除部分 flash。

  可以通过 `--range` 传入擦除的范围，如果不传将擦除负载元数据（payload meta）。

## 引导程序设计

### 存储

|     Stage    |    Memory   | Flash
|--------------|-------------|--------
|      SPL     |    0x2_0000 |    0x0
| Payload Meta |    0x2_40c8 | 0x8000
|      SEE     | 0x4000_0000 | 0x9000
|    KERNEL    | 0x4020_0000 | +sizeof(SEE)/4k
|      DTB     |      *      | +sizeof(KERNEL)/4k

> DTB 被放在 DTB 描述的物理内存区域的最后一个 2 MiB 页上，同时偏移存入 Meta
