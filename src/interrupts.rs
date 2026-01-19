//! GwenOS 中断处理模块
//!
//! 提供 IDT（中断描述符表）的初始化和异常处理功能
//!
//! # 什么是中断？
//! 中断是 CPU 暂停当前工作去处理紧急事件的机制，分为：
//! - 异常（Exception）：CPU 自己触发，如除零、缺页
//! - 硬件中断（IRQ）：外部设备触发，如键盘、定时器
//! - 软件中断：程序主动触发，如系统调用

use crate::serial;
use x86_64::structures::idt::{self, InterruptDescriptorTable, InterruptStackFrame};

// =============================================================================
// IDT 静态实例
// =============================================================================

/// 使用 lazy_static 创建静态 IDT
///
/// 为什么用 lazy_static？
/// - IDT 需要在程序启动后初始化（不是编译时）
/// - 但又需要是 'static 生命周期（永久存在）
/// - 我们使用简单的 Option 来实现延迟初始化

/// IDT 实例，使用 Mutex 保护
/// 在单核情况下其实不需要锁，但这是好习惯
static mut IDT: Option<InterruptDescriptorTable> = None;

// =============================================================================
// 异常处理函数
// =============================================================================

/// 断点异常处理器（中断号 3）
///
/// 当 CPU 执行 `int3` 指令时触发
/// 这是调试器使用的断点机制
///
/// # 参数
/// - `stack_frame`: 包含中断发生时的 CPU 状态
extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    serial::write_line("");
    serial::write_line("===========================================");
    serial::write_line("  EXCEPTION: Breakpoint (int3)");
    serial::write_line("===========================================");
    serial::write_line("");

    // 打印指令指针（发生中断的位置）
    serial::write_string("  Instruction Pointer: ");
    // 注意：这里简化处理，实际应该格式化打印地址
    serial::write_line("(see QEMU for details)");

    serial::write_line("");
    serial::write_line("  Breakpoint handled, continuing...");
    serial::write_line("===========================================");
    serial::write_line("");
}

/// 双重故障异常处理器（中断号 8）
///
/// 当处理一个异常时又发生异常，就会触发双重故障
/// 这通常意味着内核有严重 bug
///
/// 注意：双重故障是"发散"的（diverging），不能返回
extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial::write_line("");
    serial::write_line("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    serial::write_line("  EXCEPTION: Double Fault!");
    serial::write_line("  This is a critical error.");
    serial::write_line("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    serial::write_line("");

    // 双重故障无法恢复，进入无限循环
    loop {
        x86_64::instructions::hlt();
    }
}

// =============================================================================
// IDT 初始化
// =============================================================================

/// 初始化中断描述符表（IDT）
///
/// 这个函数：
/// 1. 创建一个新的 IDT
/// 2. 注册异常处理函数
/// 3. 加载 IDT 到 CPU
pub fn init() {
    serial::write_line("[DEBUG] Initializing IDT...");

    // 创建新的 IDT
    let mut idt = InterruptDescriptorTable::new();

    // 注册断点异常处理器（中断号 3）
    idt.breakpoint.set_handler_fn(breakpoint_handler);

    // 注册双重故障处理器（中断号 8）
    idt.double_fault.set_handler_fn(double_fault_handler);

    // 将 IDT 存储到静态变量中
    // 这是必须的，因为 CPU 需要 IDT 永久存在
    unsafe {
        IDT = Some(idt);

        // 加载 IDT 到 CPU
        // lidt 指令告诉 CPU IDT 的位置
        match IDT {
            Some(ref idt) => {
                idt.load();
            }
            None => {}
        }
    }

    serial::write_line("[DEBUG] IDT initialized successfully!");
}

// =============================================================================
// 测试函数
// =============================================================================

/// 触发断点异常测试
///
/// 执行 `int3` 指令来测试断点处理器是否正常工作
pub fn test_breakpoint() {
    serial::write_line("[DEBUG] Triggering breakpoint exception...");

    // int3 指令触发断点异常
    x86_64::instructions::interrupts::int3();

    serial::write_line("[DEBUG] Returned from breakpoint exception!");
}
