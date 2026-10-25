//! Browser logging for diagnostics and non-authoritative operator audit traces.
#[cfg(target_arch = "wasm32")]
struct ConsoleLogger;
#[cfg(target_arch = "wasm32")]
impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let message = wasm_bindgen::JsValue::from_str(&format!(
            "{} {}: {}",
            record.level(),
            record.target(),
            record.args()
        ));
        match record.level() {
            log::Level::Error => web_sys::console::error_1(&message),
            log::Level::Warn => web_sys::console::warn_1(&message),
            _ => web_sys::console::info_1(&message),
        }
    }
    fn flush(&self) {}
}
#[cfg(target_arch = "wasm32")]
pub fn init() {
    static LOGGER: ConsoleLogger = ConsoleLogger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}
