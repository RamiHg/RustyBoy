use crate::cpu::{Output, Result, SideEffect};

/// Generic interface to the system as a whole.
///
/// Defined mainly to enable easier testing.
pub trait System {
    fn execute_cpu_cycle(&mut self) -> Result<Output>;
    fn commit_memory_write(&mut self, raw_address: i32, value: i32);

    fn execute_machine_cycle(&mut self) -> Result<Output> {
        let cpu_output = self.execute_cpu_cycle()?;
        if let Some(SideEffect::Write { raw_address, value }) = cpu_output.side_effect {
            // In the future, this will wait until the GPU has reached the end of its 4th cycle, and
            // then commited this write.
            self.commit_memory_write(raw_address, value);
        }
        // TODO: Figure out what we should return here. Doesn't make sense to expose instruction
        // side effect.
        Ok(cpu_output)
    }
}
