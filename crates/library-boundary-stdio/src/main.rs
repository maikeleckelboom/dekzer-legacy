#![deny(unsafe_code)]

use std::process;

fn main() {
    install_panic_hook();

    if let Err(error) = library_boundary_stdio::run_main(std::env::args().skip(1)) {
        for line in error.render_lines() {
            eprintln!("{line}");
        }

        process::exit(1);
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[library-boundary-stdio] panic: {panic_info}");
    }));
}
