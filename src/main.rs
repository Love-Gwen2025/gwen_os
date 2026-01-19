//! GwenOS - 一个用 Rust 编写的 Mini 操作系统
//!
//! 这是内核的入口点文件

#![no_std] // 不链接 Rust 标准库（std），因为标准库依赖操作系统功能
#![no_main] // 禁用常规的 main 入口点，自定义入口
#![feature(abi_x86_interrupt)] // 启用 x86 中断调用约定（实验性特性）

// 引入模块
mod interrupts; // 中断处理
mod serial; // 串口输出
mod vga; // VGA 文本模式输出

use core::panic::PanicInfo;

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
    serial_println!("[DEBUG] Serial port initialized!");
    serial_println!("[DEBUG] GwenOS kernel starting...");

    // =========================================
    // 2. 初始化中断处理（IDT）
    // =========================================
    interrupts::init();

    // 测试断点异常
    interrupts::test_breakpoint();

    // =========================================
    // 3. 清空屏幕
    // =========================================
    vga::clear_screen();
    serial_println!("[DEBUG] Screen cleared");

    // =========================================
    // 4. 在屏幕中央显示欢迎信息
    // =========================================
    let welcome = "Hello, GwenOS!";
    let col = (vga::VGA_WIDTH - welcome.len()) / 2;
    let row = vga::VGA_HEIGHT / 2;

    // 使用新的 VGA 模块（绿色文字 0x0a）
    vga::write_string_at(welcome, row, col, 0x0a);
    serial_println!("[DEBUG] Displayed: {}", welcome);

    // 显示版本信息（灰色文字 0x07）
    let version = "Version 0.1.0 - Made with Rust";
    let version_col = (vga::VGA_WIDTH - version.len()) / 2;
    vga::write_string_at(version, row + 2, version_col, 0x07);
    serial_println!("[DEBUG] Displayed version info");

    // =========================================
    // 5. 演示格式化输出功能
    // =========================================
    serial_println!();
    serial_println!("=================================");
    serial_println!("  GwenOS Serial Output Working!");
    serial_println!("  Now with format support: {}", 42);
    serial_println!("=================================");
    serial_println!();

    // 使用 VGA println 宏测试
    println!(); // 换行
    println!("Kernel loaded successfully!");
    println!("Format test: 0x{:x}", 0xDEADBEEF_u32);

    serial_println!("[DEBUG] Entering main loop...");

    // =========================================
    // 6. 内核主循环
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
/// 由于我们没有操作系统支持，只能将错误信息打印到屏幕和串口
///
/// # 参数
/// - `info`: 包含 panic 信息的结构体
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 输出到串口（方便调试）
    serial_println!();
    serial_println!("!!! KERNEL PANIC !!!");
    serial_println!("{}", info);

    // 在屏幕顶部显示红色的 PANIC 信息
    vga::write_string_at("!!! KERNEL PANIC !!!", 0, 0, 0x4f); // 红底白字

    // 如果有位置信息，显示出来
    if let Some(location) = info.location() {
        vga::write_string_at("At: ", 1, 0, 0x0c); // 红色文字
        vga::write_string_at(location.file(), 1, 4, 0x0c);
    }

    // 如果有消息，显示出来
    if let Some(message) = info.message().as_str() {
        vga::write_string_at("Msg: ", 2, 0, 0x0c);
        vga::write_string_at(message, 2, 5, 0x0c);
    }

    // panic 后进入无限循环
    loop {
        x86_64_hlt();
    }
}
