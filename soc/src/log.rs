#[derive(Default)]
pub struct LogSettings {
    pub audio: bool,
    pub interrupts: bool,
    pub disassembly: bool,
    pub timer: bool,
    pub dma: bool,
    pub gpu: bool,
}

pub fn setup_logging(settings: LogSettings) -> Result<(), fern::InitError> {
    let mut allowed_modules = std::collections::HashSet::new();
    if settings.audio {
        allowed_modules.insert("audio");
    }
    if settings.interrupts {
        allowed_modules.insert("int");
    }
    if settings.disassembly && cfg!(feature = "disas") {
        allowed_modules.insert("disas");
    }
    if settings.timer {
        allowed_modules.insert("timer");
    }
    if settings.dma {
        allowed_modules.insert("dma");
    }
    if settings.gpu {
        allowed_modules.insert("gpu");
    }
    if !allowed_modules.is_empty() {
        fern::Dispatch::new()
            .filter(move |metadata| allowed_modules.contains(metadata.target()))
            .format(|out, message, record| {
                out.finish(format_args!("[{}]: {}", record.target(), message))
            })
            .level(log::LevelFilter::Trace)
            .chain(std::io::stdout())
            .apply()?;
    }
    Ok(())
}
