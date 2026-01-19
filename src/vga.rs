//! GwenOS VGA 文本模式驱动模块
//!
//! 提供 VGA 文本模式的安全输出功能
//! 使用 volatile 确保写入不被编译器优化掉

use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// =============================================================================
// VGA 常量定义
// =============================================================================

/// VGA 文本模式缓冲区的内存地址
/// 标准的 VGA 文本模式缓冲区起始地址
const VGA_BUFFER_ADDR: usize = 0xb8000;

/// VGA 文本模式的屏幕宽度（字符数）
pub const VGA_WIDTH: usize = 80;

/// VGA 文本模式的屏幕高度（行数）
pub const VGA_HEIGHT: usize = 25;

// =============================================================================
// VGA 颜色定义
// =============================================================================

/// VGA 颜色枚举
/// 表示 VGA 文本模式支持的 16 种颜色
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// 颜色代码，包含前景色和背景色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    /// 创建新的颜色代码
    ///
    /// # 参数
    /// - `foreground`: 前景色（文字颜色）
    /// - `background`: 背景色
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// =============================================================================
// VGA 字符和缓冲区结构
// =============================================================================

/// 屏幕上的一个字符
/// 包含 ASCII 字符和颜色属性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    /// ASCII 字符码
    ascii_character: u8,
    /// 颜色属性（前景色 + 背景色）
    color_code: ColorCode,
}

/// VGA 文本缓冲区
/// 使用 Volatile 包装确保写入不被优化
#[repr(transparent)]
struct Buffer {
    /// 字符数组：25行 × 80列
    chars: [[Volatile<ScreenChar>; VGA_WIDTH]; VGA_HEIGHT],
}

// =============================================================================
// Writer 结构体
// =============================================================================

/// VGA 文本写入器
/// 管理当前光标位置和颜色
pub struct Writer {
    /// 当前列位置
    column_position: usize,
    /// 当前行位置
    row_position: usize,
    /// 当前使用的颜色代码
    color_code: ColorCode,
    /// VGA 缓冲区的可变引用
    buffer: &'static mut Buffer,
}

impl Writer {
    /// 写入单个字节
    ///
    /// # 参数
    /// - `byte`: 要写入的字节（ASCII 字符）
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            // 换行符：移动到下一行
            b'\n' => self.new_line(),
            // 可打印 ASCII 字符
            byte => {
                // 如果当前行已满，换行
                if self.column_position >= VGA_WIDTH {
                    self.new_line();
                }

                let row = self.row_position;
                let col = self.column_position;

                // 使用 volatile 写入确保不被优化
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                });

                self.column_position += 1;
            }
        }
    }

    /// 写入字符串
    ///
    /// # 参数
    /// - `s`: 要写入的字符串
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // 可打印 ASCII 字符或换行符
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // 不可打印字符用 ■ 表示
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// 在指定位置写入字符串
    ///
    /// # 参数
    /// - `s`: 要写入的字符串
    /// - `row`: 行号（0-24）
    /// - `col`: 列号（0-79）
    /// - `color`: 颜色代码
    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, color: ColorCode) {
        // 边界检查：确保不超出屏幕范围
        if row >= VGA_HEIGHT {
            return;
        }

        let mut current_col = col;

        for byte in s.bytes() {
            // 边界检查：确保不超出当前行
            if current_col >= VGA_WIDTH {
                break;
            }

            let char_to_write = match byte {
                0x20..=0x7e => byte,
                _ => 0xfe, // 不可打印字符用 ■ 表示
            };

            // 使用 volatile 写入
            self.buffer.chars[row][current_col].write(ScreenChar {
                ascii_character: char_to_write,
                color_code: color,
            });

            current_col += 1;
        }
    }

    /// 换行处理
    fn new_line(&mut self) {
        // 如果不是最后一行，直接下移
        if self.row_position < VGA_HEIGHT - 1 {
            self.row_position += 1;
        } else {
            // 最后一行，滚动屏幕
            self.scroll();
        }
        self.column_position = 0;
    }

    /// 屏幕滚动
    /// 将所有行上移一行，最后一行清空
    fn scroll(&mut self) {
        // 将每一行的内容复制到上一行
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        // 清空最后一行
        self.clear_row(VGA_HEIGHT - 1);
    }

    /// 清空指定行
    ///
    /// # 参数
    /// - `row`: 要清空的行号
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..VGA_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// 清空整个屏幕
    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
        self.row_position = 0;
    }

    /// 设置当前颜色
    #[allow(dead_code)]
    pub fn set_color(&mut self, color: ColorCode) {
        self.color_code = color;
    }
}

/// 实现 fmt::Write trait，支持格式化输出
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// =============================================================================
// 全局 Writer 实例
// =============================================================================

lazy_static! {
    /// 全局 VGA Writer 实例
    /// 使用 Mutex 保护，确保线程安全
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        row_position: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(VGA_BUFFER_ADDR as *mut Buffer) },
    });
}

// =============================================================================
// 公共函数接口
// =============================================================================

/// 清空屏幕
pub fn clear_screen() {
    WRITER.lock().clear_screen();
}

/// 在指定位置写入字符串
///
/// # 参数
/// - `s`: 要写入的字符串
/// - `row`: 行号
/// - `col`: 列号
/// - `color_byte`: 颜色字节（高4位背景，低4位前景）
pub fn write_string_at(s: &str, row: usize, col: usize, color_byte: u8) {
    WRITER
        .lock()
        .write_string_at(s, row, col, ColorCode(color_byte));
}

/// 用于 print! 宏的内部打印函数
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

// =============================================================================
// 打印宏
// =============================================================================

/// 向 VGA 屏幕打印格式化文本
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

/// 向 VGA 屏幕打印格式化文本并换行
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
