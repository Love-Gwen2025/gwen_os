//! GwenOS 串口（UART）驱动模块
//!
//! 提供通过 COM1 串口输出调试信息的功能
//! 串口输出会显示在运行 QEMU 的终端窗口中

use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

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
        core::arch::asm!(
            "out dx,al",
            in("dx") port,
            in("al") value,
            // 编译器优化选项
            options(nomem, nostack, preserves_flags)
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
// 串口 Writer 结构
// =============================================================================

/// 串口写入器
/// 封装串口操作
pub struct SerialWriter {
    port: u16,
}

impl SerialWriter {
    /// 创建新的串口写入器
    const fn new(port: u16) -> Self {
        SerialWriter { port }
    }

    /// 初始化串口
    ///
    /// 配置串口参数：
    /// - 波特率：115200（最大速度）
    /// - 数据位：8 位
    /// - 停止位：1 位
    /// - 无奇偶校验
    pub fn init(&self) {
        // 1. 禁用所有中断
        outb(self.port + INT_ENABLE_REG, 0x00);

        // 2. 设置波特率为 115200
        //    波特率因子 = 115200 / 目标波特率
        //    115200 baud → 因子 = 1
        outb(self.port + LINE_CTRL_REG, 0x80); // 启用 DLAB（访问波特率寄存器）
        outb(self.port + DATA_REG, 0x01); // 波特率因子低字节
        outb(self.port + INT_ENABLE_REG, 0x00); // 波特率因子高字节

        // 3. 配置线路：8位数据，1位停止，无奇偶校验
        outb(self.port + LINE_CTRL_REG, 0x03);

        // 4. 启用 FIFO，清空缓冲区，设置 14 字节触发阈值
        outb(self.port + FIFO_CTRL_REG, 0xC7);

        // 5. 设置 Modem：启用 DTR, RTS, OUT2
        outb(self.port + MODEM_CTRL_REG, 0x0B);
    }

    /// 检查串口是否可以发送数据
    #[inline(always)]
    fn is_transmit_empty(&self) -> bool {
        // 读取线路状态寄存器，检查第5位
        // 如果第5位为1，表示发送缓冲区为空，可以发送
        (inb(self.port + LINE_STATUS_REG) & 0x20) != 0
    }

    /// 发送一个字节
    pub fn write_byte(&self, byte: u8) {
        // 等待发送缓冲区为空
        while !self.is_transmit_empty() {
            // 忙等待（自旋）
        }
        // 发送字节
        outb(self.port + DATA_REG, byte);
    }

    /// 发送字符串
    pub fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// 发送一行（自动添加换行符）
    pub fn write_line(&self, s: &str) {
        self.write_string(s);
        self.write_byte(b'\n');
    }
}

/// 实现 fmt::Write trait，支持格式化输出
impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// =============================================================================
// 全局串口实例
// =============================================================================

lazy_static! {
    /// 全局串口实例
    /// 使用 Mutex 保护，确保线程安全
    pub static ref SERIAL1: Mutex<SerialWriter> = Mutex::new(SerialWriter::new(COM1_PORT));
}

// =============================================================================
// 公共函数接口（向后兼容）
// =============================================================================

/// 初始化 COM1 串口
pub fn init() {
    SERIAL1.lock().init();
}

/// 通过串口发送一个字节
#[allow(dead_code)]
pub fn write_byte(byte: u8) {
    SERIAL1.lock().write_byte(byte);
}

/// 通过串口发送字符串
pub fn write_string(s: &str) {
    SERIAL1.lock().write_string(s);
}

/// 通过串口发送一行（自动添加换行符）
pub fn write_line(s: &str) {
    SERIAL1.lock().write_line(s);
}

/// 用于 serial_print! 宏的内部打印函数
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).unwrap();
}

// =============================================================================
// 串口打印宏
// =============================================================================

/// 向串口打印格式化文本
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

/// 向串口打印格式化文本并换行
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
