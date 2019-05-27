#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Options {
    pub use_fetcher_initial_fetch: bool,

    pub num_tcycles_in_line: i32,
    pub num_hblank_delay_tcycles: i32,

    // This is the cycle where the state actually changes to TransferringToLcd.
    pub transfer_mode_start_tcycle: i32,
    // This is the cycle where we actually start the transfer logic.
    pub transfer_start_tcycle: i32,
}

impl Options {
    pub fn new() -> Options {
        Options {
            use_fetcher_initial_fetch: false,

            num_tcycles_in_line: 456,
            num_hblank_delay_tcycles: 8,

            transfer_mode_start_tcycle: 84,
            transfer_start_tcycle: 84,
        }
    }
}
