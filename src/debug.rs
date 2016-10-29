/*
 * Helper file to help with debugging:
 * Instruction trace, instruction stepping, register view, etc
*/

pub struct Debugger {

}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {}
    }

    pub fn log_instr(&self, message: String) {
        //println!("{}", message);
    }
}
