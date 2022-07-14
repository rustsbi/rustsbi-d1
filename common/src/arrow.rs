//! 打印一个能动的箭头，在死循环中表现工作状态

/// 箭头模块
pub struct Arrow<T> {
    len: usize,
    pos: usize,
    dir: bool,
    print: T,
}

/// backspace
const BS: u8 = 8;

impl<T> Arrow<T>
where
    T: Fn(&[u8]),
{
    /// 初始化，显示一个初始状态的箭头，总长度 `len`，使用 `print` 回调打印
    pub fn init(len: usize, print: T) -> Self {
        print(b"|");
        for _ in 1..len - 3 {
            print(b" ");
        }
        print(b"<<|");
        Self {
            len: len - 4,
            pos: 0,
            dir: true,
            print,
        }
    }

    /// 转移下一个状态
    pub fn next(&mut self) {
        // 光标复位
        for _ in 0..=self.len - self.pos {
            (self.print)(&[BS]);
        }
        // 转移状态
        (self.print)(if self.pos == (if self.dir { self.len } else { 0 }) {
            // 转身
            self.dir = !self.dir;
            &[BS, BS]
        } else if self.dir {
            // 右移
            self.pos += 1;
            &[BS, BS, b' ']
        } else {
            // 左移
            self.pos -= 1;
            &[BS, b' ', BS, BS, BS]
        });
        // 显示箭头
        (self.print)(if self.dir { b">>" } else { b"<<" });
        // 光标移至尾部
        for _ in 0..self.len - self.pos {
            (self.print)(b" ");
        }
        (self.print)(&[b'|']);
    }
}
