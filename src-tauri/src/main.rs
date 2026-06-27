// 本机电源助手 — 二进制入口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    battery_assistant_lib::run()
}
