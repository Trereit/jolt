use common::constants::virtual_register_index;
use tracer::{ELFInstruction, RVTraceRow, RegisterState, RV32IM};

use super::VirtualInstructionSequence;
use crate::jolt::instruction::{
    add::ADDInstruction, movsign::MOVSIGNInstruction, mulhu::MULHUInstruction,
    mulu::MULUInstruction, JoltInstruction,
};

pub struct MULHInstruction<const WORD_SIZE: usize>;

impl<const WORD_SIZE: usize> VirtualInstructionSequence for MULHInstruction<WORD_SIZE> {
    fn virtual_sequence(trace_row: RVTraceRow) -> Vec<RVTraceRow> {
        assert_eq!(trace_row.instruction.opcode, RV32IM::MULH);
        // MULH operands
        let x = trace_row.register_state.rs1_val.unwrap();
        let y = trace_row.register_state.rs2_val.unwrap();
        // MULH source registers
        let r_x = trace_row.instruction.rs1;
        let r_y = trace_row.instruction.rs2;
        // Virtual registers used in sequence
        let v_sx = Some(virtual_register_index(0));
        let v_sy = Some(virtual_register_index(1));
        let v_0 = Some(virtual_register_index(2));
        let v_1 = Some(virtual_register_index(3));
        let v_2 = Some(virtual_register_index(4));
        let v_3 = Some(virtual_register_index(5));

        let mut virtual_sequence = vec![];

        let s_x = MOVSIGNInstruction::<WORD_SIZE>(x).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::VIRTUAL_MOVSIGN,
                rs1: r_x,
                rs2: None,
                rd: v_sx,
                imm: None,
                virtual_sequence_index: Some(0),
            },
            register_state: RegisterState {
                rs1_val: Some(x),
                rs2_val: None,
                rd_post_val: Some(s_x),
            },
            memory_state: None,
        });

        let s_y = MOVSIGNInstruction::<WORD_SIZE>(y).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::VIRTUAL_MOVSIGN,
                rs1: r_y,
                rs2: None,
                rd: v_sy,
                imm: None,
                virtual_sequence_index: Some(1),
            },
            register_state: RegisterState {
                rs1_val: Some(y),
                rs2_val: None,
                rd_post_val: Some(s_y),
            },
            memory_state: None,
        });

        let xy_high_bits = MULHUInstruction::<WORD_SIZE>(x, y).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::MULHU,
                rs1: r_x,
                rs2: r_y,
                rd: v_0,
                imm: None,
                virtual_sequence_index: Some(2),
            },
            register_state: RegisterState {
                rs1_val: Some(x),
                rs2_val: Some(y),
                rd_post_val: Some(xy_high_bits),
            },
            memory_state: None,
        });

        let sx_y_low_bits = MULUInstruction::<WORD_SIZE>(s_x, y).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::MULU,
                rs1: v_sx,
                rs2: r_y,
                rd: v_1,
                imm: None,
                virtual_sequence_index: Some(3),
            },
            register_state: RegisterState {
                rs1_val: Some(s_x),
                rs2_val: Some(y),
                rd_post_val: Some(sx_y_low_bits),
            },
            memory_state: None,
        });

        let sy_x_low_bits = MULUInstruction::<WORD_SIZE>(s_y, x).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::MULU,
                rs1: v_sy,
                rs2: r_x,
                rd: v_2,
                imm: None,
                virtual_sequence_index: Some(4),
            },
            register_state: RegisterState {
                rs1_val: Some(s_y),
                rs2_val: Some(x),
                rd_post_val: Some(sy_x_low_bits),
            },
            memory_state: None,
        });

        let partial_sum = ADDInstruction::<WORD_SIZE>(xy_high_bits, sx_y_low_bits).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::ADD,
                rs1: v_0,
                rs2: v_1,
                rd: v_3,
                imm: None,
                virtual_sequence_index: Some(5),
            },
            register_state: RegisterState {
                rs1_val: Some(xy_high_bits),
                rs2_val: Some(sx_y_low_bits),
                rd_post_val: Some(partial_sum),
            },
            memory_state: None,
        });

        let result = ADDInstruction::<WORD_SIZE>(partial_sum, sy_x_low_bits).lookup_entry();
        virtual_sequence.push(RVTraceRow {
            instruction: ELFInstruction {
                address: trace_row.instruction.address,
                opcode: RV32IM::ADD,
                rs1: v_3,
                rs2: v_2,
                rd: trace_row.instruction.rd,
                imm: None,
                virtual_sequence_index: Some(6),
            },
            register_state: RegisterState {
                rs1_val: Some(partial_sum),
                rs2_val: Some(sy_x_low_bits),
                rd_post_val: Some(result),
            },
            memory_state: None,
        });
        virtual_sequence
    }
}

#[cfg(test)]
mod test {
    use ark_std::test_rng;
    use common::constants::REGISTER_COUNT;
    use rand_chacha::rand_core::RngCore;

    use crate::jolt::vm::rv32i_vm::RV32I;

    use super::*;

    #[test]
    // TODO(moodlezoup): Turn this into a macro, similar to the `jolt_instruction_test` macro
    fn mulh_virtual_sequence_32() {
        let mut rng = test_rng();

        let r_x = rng.next_u64() % 32;
        let r_y = rng.next_u64() % 32;
        let rd = rng.next_u64() % 32;

        let x = rng.next_u32() as u64;
        let y = if r_x == r_y { x } else { rng.next_u32() as u64 };
        let result = (((x as i32 as i64) * (y as i32 as i64)) >> 32) as u32;

        let mulh_trace_row = RVTraceRow {
            instruction: ELFInstruction {
                address: rng.next_u64(),
                opcode: RV32IM::MULH,
                rs1: Some(r_x),
                rs2: Some(r_y),
                rd: Some(rd),
                imm: None,
                virtual_sequence_index: None,
            },
            register_state: RegisterState {
                rs1_val: Some(x),
                rs2_val: Some(y),
                rd_post_val: Some(result as u64),
            },
            memory_state: None,
        };

        let virtual_sequence = MULHInstruction::<32>::virtual_sequence(mulh_trace_row);
        let mut registers = vec![0u64; REGISTER_COUNT as usize];
        registers[r_x as usize] = x;
        registers[r_y as usize] = y;

        for row in virtual_sequence {
            if let Some(rs1_val) = row.register_state.rs1_val {
                assert_eq!(registers[row.instruction.rs1.unwrap() as usize], rs1_val);
            }
            if let Some(rs2_val) = row.register_state.rs2_val {
                assert_eq!(registers[row.instruction.rs2.unwrap() as usize], rs2_val);
            }

            let lookup = RV32I::try_from(&row).unwrap();
            let output = lookup.lookup_entry();
            if let Some(rd) = row.instruction.rd {
                registers[rd as usize] = output;
                assert_eq!(
                    registers[rd as usize],
                    row.register_state.rd_post_val.unwrap()
                );
            } else {
                // Virtual assert instruction
                assert!(output == 1);
            }
        }

        for (index, val) in registers.iter().enumerate() {
            if index as u64 == r_x {
                // Check that r_x hasn't been clobbered
                assert_eq!(*val, x);
            } else if index as u64 == r_y {
                // Check that r_y hasn't been clobbered
                assert_eq!(*val, y);
            } else if index as u64 == rd {
                // Check that result was written to rd
                assert_eq!(*val, result as u64);
            } else if index < 32 {
                // None of the other "real" registers were touched
                assert_eq!(*val, 0);
            }
        }
    }
}
