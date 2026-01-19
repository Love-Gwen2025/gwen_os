//! GwenOS - 一个用 Rust 编写的 Mini 操作系统
//!
//! 这是内核的入口点文件

#![no_std] // 不链接 Rust 标准库（std），因为标准库依赖操作系统功能
#![no_main] // 禁用常规的 main 入口点，自定义入口

// 引入串口模块
mod serial;

use core::panic::PanicInfo;

/// VGA 文本模式缓冲区的内存地址
/// 标准的 VGA 文本模式缓冲区起始地址
//语法详解，*mut u8 可变裸指针（即可以通过指针修改数据） as *mut u8 类型转换
const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;

/// VGA 文本模式的屏幕宽度（字符数）
const VGA_WIDTH: usize = 80;

/// VGA 文本模式的屏幕高度（行数）  
const VGA_HEIGHT: usize = 25;

/// 默认的字符颜色属性
/// 0x0f = 黑底白字 (背景色 0, 前景色 f)
const DEFAULT_COLOR: u8 = 0x0f;

fn write_string(s: &str, row: usize, col: usize, color: u8) {
    let mut offset: usize = (row * VGA_WIDTH + col) * 2;

    for byte in s.bytes() {
        if offset > row * VGA_HEIGHT * VGA_WIDTH {
            break;
        }
        unsafe {
            *VGA_BUFFER.add(offset) = byte;
            *VGA_BUFFER.add(offset + 1) = color;
        }
        offset = offset + 2;
    }
}

/// 清空屏幕
///
/// 用空格填充整个 VGA 文本缓冲区
fn clear_screen() {
    for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
        let offset = i * 2;
        unsafe {
            *VGA_BUFFER.add(offset) = b' '; // 空格字符
            *VGA_BUFFER.add(offset + 1) = DEFAULT_COLOR; // 默认颜色
        }
    }
}

// ============================================================================
// 内核入口点
// ============================================================================

/// 内核入口函数
///
/// 这是 bootloader 加载内核后跳转到的第一个函数
/// 使用 `#[unsafe(no_mangle)]` 确保函数名不被修改，以便链接器能找到它
/// 使用 `extern "C"` 确保使用 C 调用约定
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // =========================================
    // 1. 初始化串口（用于调试输出）
    // =========================================
    serial::init();
    serial::write_line("[DEBUG] Serial port initialized!");
    serial::write_line("[DEBUG] GwenOS kernel starting...");

    // =========================================
    // 2. 清空屏幕
    // =========================================
    clear_screen();
    serial::write_line("[DEBUG] Screen cleared");

    // =========================================
    // 3. 在屏幕中央显示欢迎信息
    // =========================================
    let welcome = "Hello, GwenOS!";
    let col = (VGA_WIDTH - welcome.len()) / 2;
    let row = VGA_HEIGHT / 2;

    write_string(welcome, row, col, 0x0a); // 绿色文字
    serial::write_string("[DEBUG] Displayed: ");
    serial::write_line(welcome);

    // 显示版本信息
    let version = "Version 0.1.0 - Made with Rust";
    let version_col = (VGA_WIDTH - version.len()) / 2;
    write_string(version, row + 2, version_col, 0x07); // 灰色文字
    serial::write_line("[DEBUG] Displayed version info");

    // =========================================
    // 4. 串口功能演示
    // =========================================
    serial::write_line("");
    serial::write_line("=================================");
    serial::write_line("  GwenOS Serial Output Working!");
    serial::write_line("=================================");
    serial::write_line("");
    serial::write_line("[DEBUG] Entering main loop...");

    // =========================================
    // 5. 内核主循环
    // =========================================
    loop {
        x86_64_hlt();
    }
}

/// 执行 x86_64 的 HLT 指令
///
/// HLT 指令让 CPU 暂停执行，直到下一个中断
/// 这比空循环更节能
#[inline(always)]
fn x86_64_hlt() {
    // 使用内联汇编执行 hlt 指令
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack));
    }
}

// ============================================================================
// Panic 处理
// ============================================================================

/// Panic 处理函数
///
/// 当内核发生 panic 时，这个函数会被调用
/// 由于我们没有操作系统支持，只能将错误信息打印到屏幕
///
/// # 参数
/// - `info`: 包含 panic 信息的结构体
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 在屏幕顶部显示红色的 PANIC 信息
    write_string("!!! KERNEL PANIC !!!", 0, 0, 0x4f); // 红底白字

    // 如果有位置信息，显示出来
    if let Some(location) = info.location() {
        // 创建一个简单的缓冲区来格式化文件名
        write_string("At: ", 1, 0, 0x0c); // 红色文字
        write_string(location.file(), 1, 4, 0x0c);
    }

    // 如果有消息，显示出来
    if let Some(message) = info.message().as_str() {
        write_string("Msg: ", 2, 0, 0x0c);
        write_string(message, 2, 5, 0x0c);
    }

    // panic 后进入无限循环
    loop {
        x86_64_hlt();
    }
}
