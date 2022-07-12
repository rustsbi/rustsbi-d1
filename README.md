# 哪吒引导工具

哪吒引导和引导程序调试工具。

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
