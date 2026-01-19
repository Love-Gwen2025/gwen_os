//! GwenOS 串口（UART）驱动模块
//!
//! 提供通过 COM1 串口输出调试信息的功能
//! 串口输出会显示在运行 QEMU 的终端窗口中

// =============================================================================
// 串口端口地址定义
// =============================================================================

/// COM1 串口的 I/O 端口基地址
/// 这是 PC 标准的 COM1 端口地址
const COM1_PORT: u16 = 0x3F8;

// 串口寄存器偏移量（相对于基地址）
const DATA_REG: u16 = 0; // 数据寄存器：发送/接收数据
const INT_ENABLE_REG: u16 = 1; // 中断使能寄存器
const FIFO_CTRL_REG: u16 = 2; // FIFO 控制寄存器
const LINE_CTRL_REG: u16 = 3; // 线路控制寄存器
const MODEM_CTRL_REG: u16 = 4; // Modem 控制寄存器
const LINE_STATUS_REG: u16 = 5; // 线路状态寄存器（检查是否可以发送）

// =============================================================================
// 端口 I/O 操作（x86 汇编）
// =============================================================================

/// 向指定 I/O 端口写入一个字节
///
/// # 参数
/// - `port`: I/O 端口地址
/// - `value`: 要写入的字节值
///
/// # 说明
/// 使用 x86 的 `out` 指令，这和内存写入不同！
/// 内存：直接写地址  →  *ptr = value
/// I/O：通过端口写   →  out(port, value)
#[inline(always)]
fn outb(port: u16, value: u8) {
    unsafe {
        // out 指令：将 value 写入 port 端口
        // "out dx, al" 的意思是：把 al 寄存器的值写到 dx 寄存器指定的端口
        //arm! 一个汇编宏，在rust中操作汇编指令
        core::arch::asm!(
            "out dx,al",
            in("dx") port,
            in("al") value,
            //编译器优化选项 
            options(nomem,nostack,preserves_flags)
        )
    }
}

/// 从指定 I/O 端口读取一个字节
///
/// # 参数
/// - `port`: I/O 端口地址
///
/// # 返回
/// 从端口读取的字节值
#[inline(always)]
fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        // in 指令：从 port 端口读取值到 al
        core::arch::asm!(
            "in al, dx",
            in("dx") port,    // dx = 端口地址
            out("al") value,  // al = 读取到的值
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

// =============================================================================
// 串口初始化
// =============================================================================

/// 初始化 COM1 串口
///
/// 配置串口参数：
/// - 波特率：115200（最大速度）
/// - 数据位：8 位
/// - 停止位：1 位
/// - 无奇偶校验
pub fn init() {
    // 1. 禁用所有中断
    outb(COM1_PORT + INT_ENABLE_REG, 0x00);

    // 2. 设置波特率为 115200
    //    波特率因子 = 115200 / 目标波特率
    //    115200 baud → 因子 = 1
    outb(COM1_PORT + LINE_CTRL_REG, 0x80); // 启用 DLAB（访问波特率寄存器）
    outb(COM1_PORT + DATA_REG, 0x01); // 波特率因子低字节
    outb(COM1_PORT + INT_ENABLE_REG, 0x00); // 波特率因子高字节

    // 3. 配置线路：8位数据，1位停止，无奇偶校验
    outb(COM1_PORT + LINE_CTRL_REG, 0x03);

    // 4. 启用 FIFO，清空缓冲区，设置 14 字节触发阈值
    outb(COM1_PORT + FIFO_CTRL_REG, 0xC7);

    // 5. 设置 Modem：启用 DTR, RTS, OUT2
    outb(COM1_PORT + MODEM_CTRL_REG, 0x0B);

    // 初始化完成！
}

// =============================================================================
// 串口输出函数
// =============================================================================

/// 检查串口是否可以发送数据
///
/// 通过读取线路状态寄存器的第5位判断发送缓冲区是否为空
#[inline(always)]
fn is_transmit_empty() -> bool {
    // 读取线路状态寄存器，检查第5位
    // 如果第5位为1，表示发送缓冲区为空，可以发送
    (inb(COM1_PORT + LINE_STATUS_REG) & 0x20) != 0
}

/// 通过串口发送一个字节
///
/// # 参数
/// - `byte`: 要发送的字节
pub fn write_byte(byte: u8) {
    // 等待发送缓冲区为空
    while !is_transmit_empty() {
        // 忙等待（自旋）
        // 在实际应用中可能需要加入超时机制
    }

    // 发送字节
    outb(COM1_PORT + DATA_REG, byte);
}

/// 通过串口发送字符串
///
/// # 参数
/// - `s`: 要发送的字符串
pub fn write_string(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// 通过串口发送一行（自动添加换行符）
///
/// # 参数
/// - `s`: 要发送的字符串
pub fn write_line(s: &str) {
    write_string(s);
    write_byte(b'\n');
}
