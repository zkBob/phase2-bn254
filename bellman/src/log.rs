#[allow(unused_macros)]

cfg_if! {
    if #[cfg(feature = "wasm")] {
        use web_sys;
        use web_sys::Performance;

        macro_rules! log {
            ($($t:tt)*) => (web_sys::console::log_1(&format_args!($($t)*).to_string().into()))
        }

        macro_rules! elog {
            ($($t:tt)*) => (web_sys::console::log_1(&format_args!($($t)*).to_string().into()))
        }

        macro_rules! log_verbose {
            ($($t:tt)*) => (if $crate::verbose_flag() { web_sys::console::log_1(&format_args!($($t)*).to_string().into()) })
        }

        macro_rules! elog_verbose {
            ($($t:tt)*) => (if $crate::verbose_flag() { web_sys::console::log_1(&format_args!($($t)*).to_string().into()) })
        }

    } else {
        macro_rules! log {
            ($($t:tt)*) => (println!($($t)*))
        }

        macro_rules! elog {
            ($($t:tt)*) => (eprintln!($($t)*))
        }

        macro_rules! log_verbose {
            ($($t:tt)*) => (if $crate::verbose_flag() { println!($($t)*) })
        }

        macro_rules! elog_verbose {
            ($($t:tt)*) => (if $crate::verbose_flag() { eprintln!($($t)*) })
        }
    }
}
