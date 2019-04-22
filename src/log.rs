#[derive(Default)]
pub struct LogSettings {
    pub interrupts: bool,
    pub disassembly: bool,
    pub timer: bool,
}

pub fn setup_logging(settings: LogSettings) -> Result<(), fern::InitError> {
    if settings.interrupts || settings.disassembly || settings.timer {
        fern::Dispatch::new()
            .filter(move |metadata| {
                if metadata.target() == "disas" {
                    settings.disassembly
                } else if metadata.target() == "int" {
                    settings.interrupts
                } else if metadata.target() == "timer" {
                    settings.timer
                } else {
                    true
                }
            })
            .format(|out, message, record| {
                let msg = format!("{}", message);
                out.finish(format_args!("[{}]: {}", record.target(), message))
            })
            .level(log::LevelFilter::Trace)
            .chain(std::io::stdout())
            .apply()?;
    }
    Ok(())
}
