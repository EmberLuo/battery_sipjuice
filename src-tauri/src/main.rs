// Battery SipJuice — 二进制入口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match battery_sipjuice_lib::handle_privileged_cli(std::env::args()) {
        Ok(true) => return,
        Ok(false) => {}
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
    battery_sipjuice_lib::run()
}
