pub mod loader;

use super::alu::BinaryOp;
use super::register::Register;

struct Instruction {}

enum MemoryMode {
  DontCare,
  Read,
  Write,
}

enum InternalBusDriver {
  DontCare,
  Register,
  Memory,
}

enum IncrementMode {
  Increment,
  Decrement,
  Move,
}

#[derive(Default)]
struct MemoryControl {
  mode: Option<MemoryMode>,
  address_source: Option<Register>,
}

#[derive(Default)]
struct AluControl {
  op: Option<BinaryOp>,
  act_write: bool,
  tmp_write: bool,
}

#[derive(Default)]
struct IncrementerControl {
  mode: Option<IncrementMode>,
  directly_sample_address_bus: bool,
}

#[derive(Default)]
struct MicroCode {
  // T=1 logical grouping.
  memory_control: MemoryControl,

  // T=2 logical grouping.
  register_select: Option<Register>,

  // T=3 logical grouping.
  alu_control: AluControl,

  // T=4 logical grouping.
  incrementer_control: IncrementerControl,
}
