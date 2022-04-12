import ida_allins
import ida_bytes
import ida_funcs
import ida_nalt
import idaapi
import idautils
import idc
import sys

from keystone import Ks, KS_ARCH_X86, KS_MODE_64
from miasm.analysis.data_flow import get_memlookup
from miasm.analysis.depgraph import DependencyGraph
from miasm.analysis.machine import Machine
from miasm.arch.x86.lifter_model_call import LifterModelCall_x86_64
from miasm.core.bin_stream_ida import bin_stream_ida
from miasm.core.locationdb import LocationDB
from miasm.core.utils import encode_hex
from miasm.expression.expression import ExprCompose, ExprCond, ExprId, ExprInt, ExprLoc, ExprMem, ExprOp, ExprSlice
from miasm.expression.simplifications import ExpressionSimplifier
from miasm.ir.symbexec import SymbolicExecutionEngine
from miasm.ir.translators.translator import Translator
from miasm.ir.translators.z3_ir import Z3Mem
from z3 import *

# workaround for ida breaking z3
sys.stdout.encoding = "utf-8"

# yes, this is hacky...
# (block_addr, switch_jump_addr)
SWITCH_CASES = {
    0x18039F36C: 0x18039F381,
    0x180368AEC: 0x180368AFA,
    0x1802F1C8A: 0x1802F1C98,
    0x180474009: 0x180474017,
    0x180277178: 0x180277189,
    0x1802C68BC: 0x1802C68CA,
}

readonly_regions = []
patch_queue = []
failed_blocks = []

sections = {}
# get the ranges of the sections for later checks
for x in idautils.Segments():
    sections[idc.get_segm_name(x)] = (idc.get_segm_start(x), idc.get_segm_end(x))

# some helpers for miasm
bs = None
def is_addr_ro_variable(bs, addr, size):
    if size != 64:
        return False
    in_range = False
    for x in readonly_regions:
        if x[0] <= addr and x[1] >= addr:
            in_range = True
            break
    if not in_range:
        print(f"!!! out of range access {hex(addr)} (size {size // 8})")
        return False
    try:
        _ = bs.getbytes(addr, size // 8)
    except IOError:
        return False
    return True
def read_mem(bs, expr):
    ptr = int(expr.ptr)
    var_bytes = bs.getbytes(ptr, expr.size // 8)[::-1]
    try:
        value = int(encode_hex(var_bytes), 16)
    except ValueError:
        return expr
    return ExprInt(value, expr.size)
def simp_concrete_mem(_, expr):
    # try to concretize memory lookups
    mems = get_memlookup(expr, bs, is_addr_ro_variable)
    expr_new = expr
    if mems:
        replace = {}
        for mem in mems:
            value = read_mem(bs, mem)
            replace[mem] = value
        expr_new = expr.replace_expr(replace)
    return expr_new
def simp_expand_eq(_, expr):
    # from: ExprCompose(ExprOp('==', ExprSlice(ExprId('RCX', 64), 0, 32), ExprInt(0x1, 32)), ExprInt(0x0, 31))
    # to: ExprCond(ExprOp('==', ExprSlice(ExprId('RCX', 64), 0, 32), ExprInt(0x1, 32), ExprInt(0x0, 32))
    if len(expr.args) != 2:
        return expr
    if (not isinstance(expr.args[0], ExprOp) and not isinstance(expr.args[0], ExprId)) or expr.args[0].size != 1:
        return expr
    allowed_sizes = [8, 16, 32, 64]
    if not isinstance(expr.args[1], ExprInt) or (not expr.args[1].size + 1 in allowed_sizes) or int(expr.args[1]) != 0:
        return expr
    return ExprCond(expr.args[0], ExprInt(1, expr.args[1].size + 1), ExprInt(0, expr.args[1].size + 1))
def simp_remove_useless_comparison(_, expr):
    # from: ExprOp('==', ExprId('zf', 1), ExprInt(0x1, 1))
    # to: ExprId('zf', 1)
    if len(expr.args) != 2:
        return expr
    if expr.op != "==":
        return expr
    if expr.args[0].size != 1 or expr.args[1].size != 1:
        return expr
    if not isinstance(expr.args[1], ExprInt) or expr.args[1].arg != 1:
        return expr
    return expr.args[0]
def simp_and_to_cond(_, expr):
    # from: ExprOp('&', ExprId('ECX', 32), ExprInt(0x1, 32))
    # to: ExprCond(ExprId('ECX', 32), ExprInt(0x1, 32), ExprInt(0x0, 32))
    if len(expr.args) != 2:
        return expr
    if expr.op != "&":
        return expr
    if not isinstance(expr.args[1], ExprInt) and expr.args[1] != 1:
        return expr
    return ExprCond(expr.args[0], ExprInt(1, expr.args[0].size), ExprInt(0, expr.args[0].size))
CUSTOM_PASSES = {
    ExprMem: [simp_concrete_mem],
    ExprCompose: [simp_expand_eq],
    ExprOp: [simp_remove_useless_comparison, simp_and_to_cond]
}

# get the range of the function's jump table
# the jump table is stored in .data and not .rdata, so the section is writable
# other important control-flow related data is stored here, so i can't just concretize the whole section
# luckily, the function's jump table is contiguous and only has some simple arithmetic applied to it
def get_jump_table(func):
    # find a lea instruction to work off of
    possible_targets = []
    for ea in idautils.Heads(func.start_ea, func.end_ea):
        insn = idaapi.insn_t()
        idaapi.decode_insn(insn, ea)

        if insn.itype == ida_allins.NN_lea:
            possible_targets.append(insn.ops[1].addr)
    target = 0
    print(possible_targets)
    for x in possible_targets:
        data = idaapi.get_qword(x)
        if data < sections[".text"][0] or data > sections[".text"][1]:
            print(f"skipping {hex(x)} because its data is not within .text")
            continue
        target = x
        break
    if target == 0:
        raise Exception("failed to get a target lea instruction")
    
    # start searching
    print(f"searching at {hex(target)}")
    start = target
    end = target
    """
    while True:
        start -= 8
        data = idaapi.get_qword(start)
        if data < sections[".text"][0] or data > sections[".text"][1]:
            start += 8
            break
    """
    while True:
        end += 8
        data = idaapi.get_qword(end)
        # sometimes there will just be a random bogus entry that's just a 2 byte number in a qword
        # this is just a guess on how to handle it properly, i pulled these numbers out of my ass
        if (data < sections[".text"][0] or data > sections[".text"][1]) and (data > 0xFFFF or data < 0x500):
            end -= 8
            break
    return (start, end)

# symbolically explore the function to find all basic blocks and what they point to
def explore_function(start_ea):
    loc_db = LocationDB()
    #bs = bin_stream_ida()
    machine = Machine("x86_64")
    mdis = machine.dis_engine(bs, loc_db=loc_db)

    # symbolic execution is used to solve for the possible jump addresses at the end of each basic block
    # each basic block is evaluated independently to avoid it taking a really long time
    # some loss of precision is fine because the hexrays decompiler will optimize it out anyways
    lifter = LifterModelCall_x86_64(mdis.loc_db)
    cse = SymbolicExecutionEngine(lifter)
    initial_state = cse.get_state()

    # initialize the solver
    set_param('parallel.enable', True)
    set_param("parallel.threads.max", 8)
    z3_mem = Z3Mem()
    s = SolverFor("QF_BV")
    solved_addr = BitVec("solved_addr", 64)
    for x in readonly_regions:
        for y in range(x[0], x[1] + 8, 8):
            var_bytes = bs.getbytes(y, 8)[::-1]
            value = int(encode_hex(var_bytes), 16)
            s.add(z3_mem.get(BitVecVal(y, 64), 64) == value)
    
    # explore!
    cmov_skips = 0
    branches_to_explore = [start_ea]
    unconditional_blocks = {}
    conditional_blocks = {}
    asmcfg = None
    while len(branches_to_explore) > 0:
        target = branches_to_explore.pop()
        if target in unconditional_blocks or target in conditional_blocks or target in failed_blocks:
            print(f"{hex(target)} already explored, skipping")
            continue
        #cse.set_state(state)
        # better to explore too many blocks than to have it take ages
        cse.set_state(initial_state)
        print(f"exploring {hex(target)}")
        asmcfg = mdis.dis_multiblock(target)
        #print("--- ASMCFG ---")
        #print(asmcfg)
        ircfg = lifter.new_ircfg_from_asmcfg(asmcfg)

        simp = ExpressionSimplifier()
        simp.enable_passes(ExpressionSimplifier.PASS_COMMONS)
        simp.enable_passes(CUSTOM_PASSES)

        simp_addr = simp(cse.run_block_at(ircfg, target))
        if isinstance(simp_addr, ExprInt):
            dst = int(simp_addr)
            print(f"pushing unconditional {hex(dst)}")
            branches_to_explore.append(dst)
            unconditional_blocks[target] = dst
        elif isinstance(simp_addr, ExprCond) and isinstance(simp_addr.src1, ExprInt) and isinstance(simp_addr.src2, ExprInt):
            src1 = int(simp_addr.src1)
            src2 = int(simp_addr.src2)
            print(f"pushing conditional {hex(src1)} or {hex(src2)}")
            branches_to_explore.append(src1)
            branches_to_explore.append(src2)
            conditional_blocks[target] = [src1, src2]
        elif isinstance(simp_addr, ExprLoc):
            # TODO: figure this out!!!
            print("!!! ExprLoc without offset WTF")
            failed_blocks.append(target)
            continue
        elif asmcfg.loc_key_to_block(loc_db.get_offset_location(target)).lines[-1].name == "RET":
            print("function end hit")
            continue
        elif target in SWITCH_CASES:
            # https://reverseengineering.stackexchange.com/questions/13358/how-to-find-all-switch-jump-tables-in-idapython
            print("doing switch case hack!")
            info = ida_nalt.get_switch_info(SWITCH_CASES[target])
            if info is None:
                raise Exception("failed to get switch case info")
            print(f"base: {hex(info.elbase)}")
            elem_size = info.get_jtable_element_size()
            for i in range(info.get_jtable_size()):
                branches_to_explore.append(int.from_bytes(ida_bytes.get_bytes(info.jumps + (i * elem_size), elem_size), 'little', signed=True) + info.elbase)
        else:
            # miasm really really hates cmov
            # it splits it into another ir block, which makes everything a pain in the ass
            # i might just locally merge in https://github.com/cea-sec/miasm/pull/634 to get this to work
            # since this game doesn't rely on any behavior that would be wrong if it were to be emulated like this
            cmov_found = False
            for x in asmcfg.loc_key_to_block(loc_db.get_offset_location(target)).lines:
                if "CMOV" in x.name:
                    cmov_found = True
                    break
            if cmov_found:
                print("!!! cmov found, skipping")
                cmov_skips += 1
                failed_blocks.append(target)
                continue

            print(f"all shortcuts failed, using z3! ({hex(target)})")

            print("--- miasm ir ---")
            print(simp_addr)
            
            translator = Translator.to_language("z3")
            #print("--- translated ---")
            z3_expr = translator.from_expr(simp_addr)
            #print(z3_expr)

            #print("--- block asm --- ")
            #print(asmcfg.loc_key_to_block(loc_db.get_offset_location(target)))

            #print("--- ircfg --- ")
            #print(ircfg)

            s.push() # we want to restore the original solver state after this

            solved_addr = BitVec("solved_addr", 64)
            s.add(z3_expr == solved_addr)

            i = 0 # just in case
            solutions = []
            print("start checking")
            while s.check() == sat:
                #print(s.model())
                sol = s.model()[solved_addr]
                solution = sol.as_long()
                solutions.append(solution)
                print("got solution: " + hex(solution))
                if solution & 0xFFFFFFFFF0000000 != 0x180000000:
                    raise Exception("got a bogus solution")
                s.add(z3_expr != sol)
                i += 1
                if i > 100:
                    raise Exception("too many solutions, did you forget to concretize memory?")
            if len(solutions) == 0:
                raise Exception("no solutions for block")
            
            if len(solutions) == 1:
                unconditional_blocks[target] = solutions[0]
            else:
                conditional_blocks[target] = solutions
            
            for x in solutions:
                branches_to_explore.append(x)
            
            s.pop() # restore the old solver state
    print(f"{len(failed_blocks)} failed blocks!!!!")
    print(failed_blocks)
    return (unconditional_blocks, conditional_blocks)

# patch the unconditional blocks
# this won't do anything if there are no blocks with opaque predicates
def patch_unconditional_blocks(todo):
    loc_db = LocationDB()
    #bs = bin_stream_ida()
    machine = Machine("x86_64")
    mdis = machine.dis_engine(bs, loc_db=loc_db)
    for block_start, block_target in todo.items():
        # get the asm blocks for the basic block
        y = mdis.dis_block(block_start)

        # look for the block that hits the indirect jump
        if not (y.lines[-1].name == "JMP" and isinstance(y.lines[-1].args[0], ExprId)):
            continue
        print(f"processing block at {hex(block_start)}")
        block_end = y.lines[-1].offset + 2 # register jmp is 2 bytes long
        print(f"block ends at {hex(block_end)}")

        # a relative jmp is larger than a register jmp, so we need to make space
        # backtrack and find the add instruction
        add_offset = None
        add_size = None
        for z in reversed(range(len(y.lines))):
            if y.lines[z].name == "ADD" and y.lines[z].args[0] == y.lines[-1].args[0]:
                add_offset = y.lines[z].offset
                add_size = y.lines[z + 1].offset - add_offset
                break
        if add_offset == None:
            print("did not find the add instruction, skipping")
            continue
        print(f"found add instr {hex(add_offset)}, size {hex(add_size)}")
        print(f"jmp offset {hex(block_end)}")
        
        # get all instructions in between
        extra_bytes = idaapi.get_bytes(add_offset + add_size, (block_end - 2) - (add_offset + add_size))
        if extra_bytes == None:
            extra_bytes = b""
        print(f"{len(extra_bytes)} extra bytes")

        # assemble the replacement instruction
        # the patch goes from the add instruction to the end of the block
        ks = Ks(KS_ARCH_X86, KS_MODE_64)
        patch = extra_bytes
        patch += bytes(ks.asm(f"jmp {hex(block_target - (add_offset + len(patch)))}")[0])

        # add some padding to be fancy
        patch += b"\xCC" * (block_end - add_offset - len(patch))
        print(f"patch: {patch}")

        #ida_bytes.patch_bytes(add_offset, patch)
        patch_queue.append((add_offset, patch))

# conditional blocks are much harder to patch
# eliminating the entire obfuscated trampoline isn't the goal here
# it's enough to just patch the bare minimum, as long as the behavior stays the same
# there are two types of branches: register-based and flag-based
# register-based conditional branches have very short blocks and have basically no other reason than to branch
# i suspect that those are always opaque, but they entirely depend on prior blocks and have no conditional logic
# flag-based conditional branches can appear at the end of any block
def patch_conditional_blocks(todo):
    loc_db = LocationDB()
    #bs = bin_stream_ida()
    machine = Machine("x86_64")
    mdis = machine.dis_engine(bs, loc_db=loc_db)
    lifter = machine.lifter_model_call(loc_db)

    for addr, targets in todo.items():
        # probably a switch case
        if len(targets) != 2:
            print(f"skipping {hex(addr)} because it has {len(targets)} targets")
            continue

        # disassemble the block
        asmcfg = mdis.dis_multiblock(addr)
        block = asmcfg.getby_offset(addr)

        # check if it's actually an obfuscated jump
        last_instr = block.lines[-1]
        if not (last_instr.name == "JMP" and isinstance(last_instr.args[0], ExprId)):
            print(f"skipping {hex(addr)} because it's not actually an obfuscated jump")
            continue
        print(f"processing conditional block at {hex(addr)}")
        target_reg = last_instr.args[0]
        print(f"targeting register {target_reg}")

        #print(block)

        # generate the dependency graph
        end_addr = last_instr.offset
        ircfg = lifter.new_ircfg_from_asmcfg(asmcfg)
        block_loc_key = next(iter(ircfg.getby_offset(end_addr)))
        ir_block = ircfg.get_block(block_loc_key)
        assignblk_index = 0
        for assignblk_index, assignblk in enumerate(ir_block):
            if assignblk.instr.offset == end_addr:
                break
        dg = DependencyGraph(ircfg, implicit=False, apply_simp=True, follow_mem=True, follow_call=False)

        # process the resulting graph
        # it should be fully linear because this is only for a single basic block
        # also, it should only return one result since only one register was specified
        graph = next(dg.get(ir_block.loc_key, [target_reg], assignblk_index, set()))
        print(f"relevant instructions for {hex(addr)}")
        instr_addrs = []
        for node in graph.relevant_nodes:
            offset = ircfg.blocks[node.loc_key][node.line_nb].instr.offset
            if not offset in instr_addrs:
                instr_addrs.append(offset)
        instr_addrs.sort()
        for x in instr_addrs:
            print(idc.GetDisasm(x))

        # dictionary of 8-bit registers to 64-bit registers for later
        lifter = LifterModelCall_x86_64(mdis.loc_db)
        regs = lifter.arch.regs
        small_to_large_reg = {
            regs.AL: regs.RAX,
            regs.BL: regs.RBX,
            regs.CL: regs.RCX,
            regs.DL: regs.RDX,
            regs.SIL: regs.RSI,
            regs.DIL: regs.RDI,
            regs.R8B: regs.R8,
            regs.R9B: regs.R9,
            regs.R10B: regs.R10,
            regs.R11B: regs.R11,
            regs.R12B: regs.R12,
            regs.R13B: regs.R13,
            regs.R14B: regs.R14,
            regs.R15B: regs.R15,

            regs.EAX: regs.RAX
        }

        # process the instructions in reverse
        # SET*-based, then SBB-based, and finally register-based
        trampoline_start_addr = 0
        force_zero_reg = None
        reg_to_check = None
        # SET*
        for x in instr_addrs[::-1]:
            instr = mdis.dis_instr(x)
            if instr.name.startswith("SET") and isinstance(instr.args[0], ExprId):
                trampoline_start_addr = x
                force_zero_reg = small_to_large_reg[instr.args[0]]
                print(f"found {instr.name} at {hex(x)}, forcing {force_zero_reg} to 0")
                break
        # SBB
        for i, x in reversed(list(enumerate(instr_addrs))):
            instr = mdis.dis_instr(x)
            prev_instr = mdis.dis_instr(instr_addrs[i - 1])
            if instr.name == "SBB" and isinstance(instr.args[0], ExprId) and isinstance(instr.args[1], ExprInt):
                if prev_instr.name == "MOV" and prev_instr.args[0] == instr.args[0] and isinstance(instr.args[1], ExprInt):
                    trampoline_start_addr = instr_addrs[i - 1]
                    print(f"found {instr.name} at {hex(x)}, forcing {force_zero_reg} to 0")
                    break
        # register
        # TODO: this sucks
        for _ in range(1):
            if trampoline_start_addr == 0:
                instr = mdis.dis_instr(instr_addrs[0])
                if (instr.name != "MOV" and instr.name != "MOVZX") or not isinstance(instr.args[0], ExprId) or not isinstance(instr.args[1], ExprId):
                    break
                # register-based is theoretically the most prone to failure, so there is a length sanity check
                if len(block.lines) > 15:
                    break
                # the mov is always between 1 byte registers
                if instr.args[1].size != 8:
                    break
                print(f"!!! register-based block at {hex(addr)}, check if it's fine!")
                #reg_to_check = small_to_large_reg[instr.args[1]]
                trampoline_start_addr = addr
                if instr.name == "MOV":
                    force_zero_reg = small_to_large_reg[instr.args[0]]
                reg_to_check = instr.args[1]

        if trampoline_start_addr == 0:
            raise Exception(f"block at {hex(addr)} did not get processed")
        
        # use symbolic execution to resolve and simplify the underlying condition
        # i can't simply start symbolic execution at an offset into a block, so the block will have to be redisassembled from that point down
        trampoline_asmcfg = mdis.dis_multiblock(trampoline_start_addr)
        trampoline_ircfg = lifter.new_ircfg_from_asmcfg(trampoline_asmcfg)

        cse = SymbolicExecutionEngine(lifter)
        simp = ExpressionSimplifier()
        simp.enable_passes(ExpressionSimplifier.PASS_COMMONS)
        simp.enable_passes(CUSTOM_PASSES)

        # inject any known assumptions into the symbolic execution engine state before running
        if not force_zero_reg is None:
            cse.symbols[force_zero_reg] = ExprInt(0, 64)
        cse_result = simp(cse.run_block_at(trampoline_ircfg, trampoline_start_addr))
        print(cse_result)
        cond_ok = isinstance(cse_result.cond, ExprId)
        if not cond_ok:
            cond_ok = isinstance(cse_result.cond, ExprOp) and len(cse_result.cond.args) == 2 and isinstance(cse_result.cond.args[0], ExprId) and isinstance(cse_result.cond.args[1], ExprId)
        if not reg_to_check is None:
            cond_ok = isinstance(cse_result.cond, ExprSlice) and isinstance(cse_result.cond.arg, ExprId) and cse_result.cond.size == 8
        if not isinstance(cse_result, ExprCond) or not cond_ok or not isinstance(cse_result.src1, ExprInt) or not isinstance(cse_result.src2, ExprInt):
            raise Exception("failed to sufficiently simplify the trampoline")
        
        # TODO: handle extraneous instructions
        extra_instrs = []
        extra_instr_bytes = b""
        # ignore the jmp at the end
        for x in trampoline_asmcfg.getby_offset(trampoline_start_addr).lines[:-1]:
            if not x.offset in instr_addrs:
                extra_instrs.append(x.offset)
        print(f"extra instructions:", [f"{hex(x)}: {idc.GetDisasm(x)}" for x in extra_instrs])
        for x in extra_instrs:
            insn = idaapi.insn_t()
            length = idaapi.decode_insn(insn, ea)
            extra_instr_bytes += idaapi.get_bytes(ea, length)
        #if len(extra_instrs) != 0:
        #    raise Exception("extraneous instructions are not supported yet")
        
        # generate the patch
        # unfortunately, this isn't nearly as simple as in the unconditional case
        # two jumps, a conditional and an unconditional one, are required
        # sometimes, a test instruction might be needed too if it's a register-based jump
        # it's also possible that the trampoline instructions could interfere with the information that has to be read
        # as such, the patch will be applied right when the flag/register is being read for the first time
        
        flag_to_jmp = {
            regs.zf: "jz",
            regs.cf: "jc",
            ExprOp('|', ExprId('cf', 1), ExprId('zf', 1)): "ja"
        }
        ks = Ks(KS_ARCH_X86, KS_MODE_64)
        patch = extra_instr_bytes
        if not reg_to_check is None:
            # jump if register is not zero
            patch += bytes(ks.asm(f"test {reg_to_check}, {reg_to_check}")[0])
            patch += bytes(ks.asm(f"jnz {hex(cse_result.src1.arg - (trampoline_start_addr + len(patch)))}")[0])
        else:
            patch += bytes(ks.asm(f"{flag_to_jmp[cse_result.cond]} {hex(cse_result.src1.arg - (trampoline_start_addr + len(patch)))}")[0])
        patch += bytes(ks.asm(f"jmp {hex(cse_result.src2.arg - (trampoline_start_addr + len(patch)))}")[0])

        # add some padding to be fancy
        # the last instruction is guaranteed to be a register jmp and those are always 2 bytes long
        patch += b"\xCC" * (last_instr.offset + 2 - trampoline_start_addr - len(patch))
        print(f"patch: {patch}")
        patch_queue.append((trampoline_start_addr, patch))

def apply_queued_patches():
    # patches are queued so development is easier
    # also, it prevents the function from being in a half-patched state if something goes wrong
    for x in patch_queue:
        ida_bytes.patch_bytes(*x)
        addr, patch_bytes = x
        idc.del_items(addr, 0, len(patch_bytes))
        for i in range(len(patch_bytes)):
            idc.create_insn(addr + i)


if __name__ == "__main__":
    bs = bin_stream_ida()

    ea = idc.get_screen_ea()
    func = ida_funcs.get_func(ea)
    print(f"function starts at {hex(func.start_ea)}")

    jump_table_start, jump_table_end = get_jump_table(func)
    print(f"jump table from {hex(jump_table_start)} to {hex(jump_table_end)}")
    readonly_regions.append((jump_table_start, jump_table_end))

    unconditional_blocks, conditional_blocks = explore_function(func.start_ea)
    print(f"{len(conditional_blocks)} conditional blocks")
    print(conditional_blocks)
    print(f"{len(unconditional_blocks)} unconditional blocks")
    print(unconditional_blocks)

    patch_unconditional_blocks(unconditional_blocks)
    patch_conditional_blocks(conditional_blocks)
    apply_queued_patches()

    print(f"{len(failed_blocks)} failed blocks!")
    print(failed_blocks)
