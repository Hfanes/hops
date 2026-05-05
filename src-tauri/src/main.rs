// Keep Hops windowless/background behavior without console flashes on URL activation.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    hops_lib::run()
}
