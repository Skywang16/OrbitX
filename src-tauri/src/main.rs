// 在Windows发布版本中防止额外的控制台窗口，请勿删除！
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    terminal_lib::run()
}
