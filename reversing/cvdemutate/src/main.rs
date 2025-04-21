#![feature(let_chains)]

use pelite::{
    FileMap,
    pe32::{Pe, PeFile},
};
use zydis::{
    Decoder, EncoderOperand, EncoderRequest, MachineMode, Mnemonic, OperandType, Register,
    StackWidth, VisibleOperands, ffi::OperandMemory,
};

const SP_REG: Register = Register::ESP;
const ADDR_SIZE: u32 = 4;
const MACHINE_MODE: MachineMode = MachineMode::LONG_COMPAT_32;
const STACK_WIDTH: StackWidth = StackWidth::_32;
const SP_MEM: OperandMemory = OperandMemory {
    base: SP_REG,
    index: Register::NONE,
    scale: 0,
    displacement: 0,
    size: ADDR_SIZE as u16,
};

#[derive(Debug, Clone)]
struct OptInstruction {
    pub orig_addr: u64,
    pub instr: EncoderRequest,
    pub should_delete: bool,
}

impl OptInstruction {
    pub fn add_imm(&self) -> Option<u32> {
        let ops = self.instr.operands();
        match self.instr.mnemonic {
            Mnemonic::ADD => {
                if ops.len() == 2 && ops[1].ty == OperandType::IMMEDIATE {
                    Some(ops[1].imm as u32)
                } else {
                    None
                }
            }
            Mnemonic::SUB => {
                if ops.len() == 2 && ops[1].ty == OperandType::IMMEDIATE {
                    Some(-(ops[1].imm as i64) as u32)
                } else {
                    None
                }
            }
            Mnemonic::INC => Some(1),
            Mnemonic::DEC => Some(!0),
            _ => None,
        }
    }
}

type OptPass = fn(&mut [OptInstruction]) -> bool;

fn get_chunk<const COUNT: usize>(
    instrs: &mut [OptInstruction],
) -> Option<&mut [OptInstruction; COUNT]> {
    instrs.split_last_chunk_mut().map(|x| x.1)
}

fn merge_adds(instrs: &mut [OptInstruction]) -> bool {
    // add op, imm1; add op, imm2 -> add op, imm1 + imm2
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };

    if let Some(imm) = chunk[1].add_imm()
        && let Some(imm2) = chunk[0].add_imm()
        && chunk[1].instr.operands()[0] == chunk[0].instr.operands()[0]
    {
        chunk[0].should_delete = true;

        let diff = imm.wrapping_add(imm2) as i32;
        if diff >= 0 {
            chunk[1].instr = EncoderRequest::new(MACHINE_MODE, Mnemonic::ADD)
                .add_operand(chunk[1].instr.operands()[0].clone())
                .add_operand(diff);
        } else {
            chunk[1].instr = EncoderRequest::new(MACHINE_MODE, Mnemonic::SUB)
                .add_operand(chunk[1].instr.operands()[0].clone())
                .add_operand(-diff);
        }
        return true;
    }

    false
}

fn remove_alu_nops(instrs: &mut [OptInstruction]) -> bool {
    // add/sub reg, 0 -> nop
    let last = &mut instrs[instrs.len() - 1];
    if last.add_imm() == Some(0) {
        last.should_delete = true;
        true
    } else {
        false
    }
}

fn normalize_pushes(instrs: &mut [OptInstruction]) -> bool {
    // push dummy/sub esp, addr_size; mov [esp], src -> push src
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };
    if chunk[1].instr.mnemonic == Mnemonic::MOV
        && chunk[1].instr.operands()[0].ty == OperandType::MEMORY
        && chunk[1].instr.operands()[0].mem == SP_MEM
        && chunk[1].instr.operands()[1].reg.value != SP_REG // Optimization isn't safe if SP is used, see normalize_push_esp
        && (chunk[0].instr.mnemonic == Mnemonic::PUSH
            || chunk[0].add_imm() == Some(!ADDR_SIZE + 1))
    {
        chunk[0].should_delete = true;
        chunk[1].instr =
            EncoderRequest::new32(Mnemonic::PUSH).add_operand(chunk[1].instr.operands()[1].clone());
        true
    } else {
        false
    }
}

fn normalize_push_esp(instrs: &mut [OptInstruction]) -> bool {
    // push dummy/sub esp, addr_size; mov [esp], esp; add dword ptr [esp], 4 -> push esp
    let Some(chunk) = get_chunk::<3>(instrs) else {
        return false;
    };
    if (chunk[0].instr.mnemonic == Mnemonic::PUSH || chunk[0].add_imm() == Some(!ADDR_SIZE + 1))
        && chunk[1].instr.mnemonic == Mnemonic::MOV
        && chunk[1].instr.operands()[0].ty == OperandType::MEMORY
        && chunk[1].instr.operands()[0].mem == SP_MEM
        && chunk[2].instr.mnemonic == Mnemonic::ADD
        && chunk[2].instr.operands()[0].mem == SP_MEM
        && chunk[2].instr.operands()[1].imm == 4
    {
        chunk[0].should_delete = true;
        chunk[1].should_delete = true;
        chunk[2].instr = EncoderRequest::new32(Mnemonic::PUSH).add_operand(SP_REG);
        true
    } else {
        false
    }
}

fn normalize_pops(instrs: &mut [OptInstruction]) -> bool {
    // mov dst, [esp]; add esp, addr_size -> pop dst
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };
    if chunk[1].add_imm() == Some(ADDR_SIZE)
        && chunk[1].instr.operands()[0].reg.value == SP_REG
        && chunk[0].instr.mnemonic == Mnemonic::MOV
        && chunk[0].instr.operands()[1].mem == SP_MEM
    {
        chunk[0].should_delete = true;
        chunk[1].instr =
            EncoderRequest::new32(Mnemonic::POP).add_operand(chunk[0].instr.operands()[0].clone());
        true
    } else {
        false
    }
}

fn push_pop_pair_mov(instrs: &mut [OptInstruction]) -> bool {
    // push src; pop dst -> mov dst, src
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic == Mnemonic::PUSH && chunk[1].instr.mnemonic == Mnemonic::POP {
        chunk[0].should_delete = true;

        chunk[1].instr = EncoderRequest::new32(Mnemonic::MOV)
            .add_operand(chunk[1].instr.operands()[0].clone())
            .add_operand(chunk[0].instr.operands()[0].clone());
        true
    } else {
        false
    }
}

fn push_pop_pair_op(instrs: &mut [OptInstruction]) -> bool {
    // push op; <something with [esp]; pop op -> <something with op>
    let Some(chunk) = get_chunk::<3>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic != Mnemonic::PUSH
        || !chunk[1].instr.operands().iter().any(|x| x.mem == SP_MEM)
        || chunk[2].instr.mnemonic != Mnemonic::POP
        || chunk[2].instr.operands()[0] != chunk[0].instr.operands()[0]
    {
        return false;
    }

    chunk[0].should_delete = true;
    chunk[2].should_delete = true;

    let target = chunk[0].instr.operands()[0].clone();
    for op in chunk[1].instr.operands_mut().iter_mut() {
        if op.mem == SP_MEM {
            *op = target.clone();
        }
    }

    true
}

fn dead_movs(instrs: &mut [OptInstruction]) -> bool {
    // mov dst, dummy; mov dst, src -> mov dst, src
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic == Mnemonic::MOV
        && chunk[1].instr.mnemonic == Mnemonic::MOV
        && chunk[0].instr.operands()[0] == chunk[1].instr.operands()[0]
    {
        chunk[0].should_delete = true;
        true
    } else {
        false
    }
}

fn alu_const_fold(instrs: &mut [OptInstruction]) -> bool {
    // So far I've only seen this on 32-bit values
    let Some(chunk) = get_chunk::<2>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic != Mnemonic::MOV
        || chunk[0].instr.operands()[1].ty != OperandType::IMMEDIATE
        || chunk[0].instr.operands()[0] != chunk[1].instr.operands()[0]
    //|| last.instr.operand_size_hint != OperandSizeHint::_32
    {
        return false;
    }

    let new_imm: Option<i32> = match chunk[1].instr.mnemonic {
        Mnemonic::ADD => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some(
                    chunk[0].instr.operands()[1]
                        .imm
                        .wrapping_add(chunk[1].instr.operands()[1].imm) as i32,
                )
            } else {
                None
            }
        }
        Mnemonic::SUB => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some(
                    chunk[0].instr.operands()[1]
                        .imm
                        .wrapping_sub(chunk[1].instr.operands()[1].imm) as i32,
                )
            } else {
                None
            }
        }
        Mnemonic::INC => Some(chunk[0].instr.operands()[1].imm.wrapping_add(1) as i32),
        Mnemonic::DEC => Some(chunk[0].instr.operands()[1].imm.wrapping_sub(1) as i32),
        Mnemonic::NEG => Some((!chunk[0].instr.operands()[1].imm).wrapping_add(1) as i32),
        Mnemonic::NOT => Some(!chunk[0].instr.operands()[1].imm as i32),
        Mnemonic::AND => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some((chunk[0].instr.operands()[1].imm & chunk[1].instr.operands()[1].imm) as i32)
            } else {
                None
            }
        }
        Mnemonic::OR => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some((chunk[0].instr.operands()[1].imm | chunk[1].instr.operands()[1].imm) as i32)
            } else {
                None
            }
        }
        Mnemonic::XOR => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some((chunk[0].instr.operands()[1].imm ^ chunk[1].instr.operands()[1].imm) as i32)
            } else {
                None
            }
        }
        Mnemonic::SHL => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some(
                    (chunk[0].instr.operands()[1].imm as u32)
                        .wrapping_shl(chunk[1].instr.operands()[1].imm as u32)
                        as i32,
                )
            } else {
                None
            }
        }
        Mnemonic::SHR => {
            if chunk[1].instr.operands()[1].ty == OperandType::IMMEDIATE {
                Some(
                    (chunk[0].instr.operands()[1].imm as u32)
                        .wrapping_shr(chunk[1].instr.operands()[1].imm as u32)
                        as i32,
                )
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(new_imm) = new_imm {
        chunk[0].should_delete = true;

        chunk[1].instr = EncoderRequest::new32(Mnemonic::MOV)
            .add_operand(chunk[1].instr.operands()[0].clone())
            .add_operand(new_imm);
        true
    } else {
        false
    }
}

fn imm_in_temp_reg(instrs: &mut [OptInstruction]) -> bool {
    // push reg; mov reg, imm; <some op with reg>; pop reg
    let Some(chunk) = get_chunk::<4>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic != Mnemonic::PUSH
        || chunk[0].instr.operands()[0].ty != OperandType::REGISTER
        || chunk[1].instr.mnemonic != Mnemonic::MOV
        || chunk[1].instr.operands()[0] != chunk[0].instr.operands()[0]
        || chunk[1].instr.operands()[1].ty != OperandType::IMMEDIATE
        || chunk[2]
            .instr
            .operands()
            .iter()
            .any(|x| x.ty == OperandType::REGISTER && x.reg.value == SP_REG)
        || chunk[3].instr.mnemonic != Mnemonic::POP
        || chunk[3].instr.operands()[0] != chunk[0].instr.operands()[0]
    {
        return false;
    }

    chunk[0].should_delete = true;
    chunk[1].should_delete = true;
    chunk[3].should_delete = true;

    let push_op = chunk[0].instr.operands()[0].clone();
    let mov_op = chunk[1].instr.operands()[1].clone();
    for op in chunk[2].instr.operands_mut().iter_mut() {
        if *op == push_op {
            *op = mov_op.clone();
        }
    }

    for op in chunk[2].instr.operands_mut().iter_mut() {
        if op.ty == OperandType::MEMORY && op.mem.base == SP_REG {
            op.mem.displacement = op.mem.displacement.wrapping_sub(4);
        }
    }

    true
}

fn dumb_xchg_push(instrs: &mut [OptInstruction]) -> bool {
    // push reg; mov reg, esp, xchg reg, [esp]; pop esp/mov esp, [esp] -> sub esp, 4
    let Some(chunk) = get_chunk::<4>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic != Mnemonic::PUSH
        || chunk[0].instr.operands()[0].ty != OperandType::REGISTER
        || chunk[1].instr.mnemonic != Mnemonic::MOV
        || chunk[1].instr.operands()[0] != chunk[0].instr.operands()[0]
        || chunk[1].instr.operands()[1].reg.value != SP_REG
        || chunk[2].instr.mnemonic != Mnemonic::XCHG // Operands are flipped for some reason?
        || chunk[2].instr.operands()[1] != chunk[0].instr.operands()[0]
        || chunk[2].instr.operands()[0].mem != SP_MEM
        || !((chunk[3].instr.mnemonic == Mnemonic::POP
            && chunk[3].instr.operands()[0].reg.value == SP_REG)
            || (chunk[3].instr.mnemonic == Mnemonic::MOV
                && chunk[3].instr.operands()[0].reg.value == SP_REG
                && chunk[3].instr.operands()[1].mem == SP_MEM))
    {
        return false;
    }

    chunk[0].should_delete = true;
    chunk[1].should_delete = true;
    chunk[2].should_delete = true;
    chunk[3].instr = EncoderRequest::new(MACHINE_MODE, Mnemonic::SUB)
        .add_operand(SP_REG)
        .add_operand(4);

    true
}

fn dumb_xchg_pop(instrs: &mut [OptInstruction]) -> bool {
    // push reg; mov reg, esp, add reg, 8; xchg reg, [esp]; pop esp/mov esp, [esp] -> sub esp, 4
    let Some(chunk) = get_chunk::<5>(instrs) else {
        return false;
    };
    if chunk[0].instr.mnemonic != Mnemonic::PUSH
        || chunk[0].instr.operands()[0].ty != OperandType::REGISTER
        || chunk[1].instr.mnemonic != Mnemonic::MOV
        || chunk[1].instr.operands()[0] != chunk[0].instr.operands()[0]
        || chunk[1].instr.operands()[1].reg.value != SP_REG
        || chunk[2].instr.mnemonic != Mnemonic::ADD
        || chunk[2].instr.operands()[0] != chunk[0].instr.operands()[0]
        || chunk[2].instr.operands()[1].imm != 8
        || chunk[3].instr.mnemonic != Mnemonic::XCHG // Operands are flipped for some reason?
        || chunk[3].instr.operands()[1] != chunk[0].instr.operands()[0]
        || chunk[3].instr.operands()[0].mem != SP_MEM
        || !((chunk[4].instr.mnemonic == Mnemonic::POP
            && chunk[4].instr.operands()[0].reg.value == SP_REG)
            || (chunk[4].instr.mnemonic == Mnemonic::MOV
                && chunk[4].instr.operands()[0].reg.value == SP_REG
                && chunk[4].instr.operands()[1].mem == SP_MEM))
    {
        return false;
    }

    chunk[0].should_delete = true;
    chunk[1].should_delete = true;
    chunk[2].should_delete = true;
    chunk[3].should_delete = true;
    chunk[4].instr = EncoderRequest::new(MACHINE_MODE, Mnemonic::ADD)
        .add_operand(SP_REG)
        .add_operand(4);

    true
}

fn xor_xchg(instrs: &mut [OptInstruction]) -> bool {
    // xor op1, op2; xor op2, op1; xor op1, op2 -> xchg op1, op2
    let Some(chunk) = get_chunk::<3>(instrs) else {
        return false;
    };
    if chunk.iter().any(|x| x.instr.mnemonic != Mnemonic::XOR)
        || chunk[0].instr.operands()[0] != chunk[1].instr.operands()[1]
        || chunk[0].instr.operands()[1] != chunk[1].instr.operands()[0]
        || chunk[1].instr.operands()[0] != chunk[2].instr.operands()[1]
        || chunk[1].instr.operands()[1] != chunk[2].instr.operands()[0]
        || chunk[0].instr.operands()[0] != chunk[2].instr.operands()[0]
        || chunk[0].instr.operands()[1] != chunk[2].instr.operands()[1]
    {
        return false;
    }

    chunk[0].should_delete = true;
    chunk[1].instr.mnemonic = Mnemonic::XCHG;
    chunk[2].should_delete = true;

    true
}

fn add_sub_pair_nop(instrs: &mut [OptInstruction]) -> bool {
    // add reg1, imm; add/sub reg1, reg2; sub reg1, imm -> add/sub reg1, reg2
    let Some(chunk) = get_chunk::<3>(instrs) else {
        return false;
    };
    if let Some(imm1) = chunk[0].add_imm()
        && let Some(imm2) = chunk[2].add_imm()
        && imm1 == (!imm2).wrapping_add(1)
        && (chunk[1].instr.mnemonic == Mnemonic::ADD || chunk[1].instr.mnemonic == Mnemonic::SUB)
        && chunk[0].instr.operands()[0] == chunk[2].instr.operands()[0]
        && chunk[1].instr.operands()[0] == chunk[0].instr.operands()[0]
    {
        chunk[0].should_delete = true;
        chunk[2].should_delete = true;
        true
    } else {
        false
    }
}

fn main() {
    //const BB_START: usize = 0x457E73;
    //const BB_END: usize = 0x457F7B;
    //const BB_START: usize = 0x473FFA;
    //const BB_END: usize = 0x47425D;
    //const BB_START: usize = 0x474304;
    //const BB_END: usize = 0x4744DE;
    //const BB_START: usize = 0x4744DE;
    //const BB_END: usize = 0x4745DB;

    let file_map = FileMap::open("/home/khangaroo/Downloads/cvtest/cvtest_protected1.exe")
        .expect("opening file");
    let pe = PeFile::from_bytes(&file_map).expect("parsing file");
    let pe_base = pe.optional_header().ImageBase as usize;

    let vlizer = pe
        .section_headers()
        .by_name(".vlizer")
        .expect("finding section");
    let vlizer_addr = pe_base + vlizer.VirtualAddress as usize;
    let vlizer_data: &[u8] = pe
        .derva_slice(vlizer.VirtualAddress, vlizer.SizeOfRawData as usize)
        .expect("getting data");

    let decoder = Decoder::new(MACHINE_MODE, STACK_WIDTH).unwrap();

    // Slightly misleading name, blocks are only split on jump targets and not jump instructions
    const BASIC_BLOCKS: &[(usize, usize)] = &[
        (0x457E73, 0x457F7B),
        (0x473FFA, 0x47425D),
        (0x47425D, 0x47427C),
        (0x47428A, 0x474304),
        (0x474304, 0x4744DE),
        (0x4744DE, 0x4745DB),
        (0x4745EB, 0x47460F),
        (0x474617, 0x47466A),
        (0x474672, 0x4746E0),
        (0x4746E0, 0x474744),
        (0x474744, 0x474762),
        (0x474762, 0x4747B6),
        (0x41D445, 0x41D54E),
    ];

    for (block_start, block_end) in BASIC_BLOCKS.iter() {
        let block_start = *block_start;
        let block_end = *block_end;

        let mut instrs = decoder
            .decode_all::<VisibleOperands>(
                &vlizer_data[block_start - vlizer_addr..block_end - vlizer_addr],
                block_start as u64,
            )
            .map(|x| {
                let x = x.expect("decoding");
                OptInstruction {
                    orig_addr: x.0,
                    instr: x.2.into(),
                    should_delete: false,
                }
            })
            .collect::<Vec<_>>();

        println!("# orig: {}", instrs.len());

        const PASSES: &[OptPass] = &[
            merge_adds,
            remove_alu_nops,
            normalize_pushes,
            normalize_push_esp,
            normalize_pops,
            push_pop_pair_mov,
            push_pop_pair_op,
            dead_movs,
            alu_const_fold,
            imm_in_temp_reg,
            dumb_xchg_push,
            dumb_xchg_pop,
            xor_xchg,
            add_sub_pair_nop,
        ];
        loop {
            let mut changed = false;
            for pass in PASSES {
                for i in 1..=instrs.len() {
                    changed |= pass(&mut instrs[..i]);
                }
                instrs.retain(|x| !x.should_delete);
            }
            if !changed {
                break;
            }
        }

        println!("# opt: {}", instrs.len());

        let mut buf = Vec::with_capacity(block_end - block_start);
        let mut last_instr_was_jmp = false;
        for instr in &mut instrs {
            // Fix relative jumps
            if ((Mnemonic::JB as u32)..=(Mnemonic::JZ as u32))
                .contains(&(instr.instr.mnemonic as u32))
                && instr.instr.operands()[0].ty == OperandType::IMMEDIATE
            {
                let diff = (instr.orig_addr - block_start as u64).wrapping_sub(buf.len() as u64);
                instr.instr.operands_mut()[0].imm =
                    instr.instr.operands()[0].imm.wrapping_add(diff);
            }

            //println!("{instr:#?}");
            instr
                .instr
                .encode_extend(&mut buf)
                .unwrap_or_else(|_| panic!("encoding {:#?}", instr.instr));

            // Fix call to get current address
            if instr.instr.mnemonic == Mnemonic::CALL && instr.instr.operands()[0].imm == 0 {
                let diff = (instr.orig_addr - block_start as u64)
                    .wrapping_sub(buf.len() as u64)
                    .wrapping_add(5);
                EncoderRequest::new(MACHINE_MODE, Mnemonic::ADD)
                    .add_operand(EncoderOperand::mem_custom(SP_MEM))
                    .add_operand(diff as u32)
                    .encode_extend(&mut buf)
                    .expect("encoding call fix");
            }

            last_instr_was_jmp = instr.instr.mnemonic == Mnemonic::JMP;
        }

        if !last_instr_was_jmp {
            let jmp_len = ((block_end - block_start) as u64).wrapping_sub(buf.len() as u64);
            EncoderRequest::new(MACHINE_MODE, Mnemonic::JMP)
                .add_operand(jmp_len.wrapping_sub(if jmp_len < 128 { 2 } else { 5 }))
                .encode_extend(&mut buf)
                .expect("encoding final jmp");
        }

        assert!(buf.len() <= block_end - block_start);

        print!("idaapi.patch_bytes(0x{block_start:X}, bytes.fromhex(\"");
        for x in &buf {
            print!("{:02X}", x);
        }
        println!("\"))");
    }
}
