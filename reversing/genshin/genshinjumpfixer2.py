from unicorn import *
from unicorn.x86_const import *

from keystone import Ks, KS_ARCH_X86, KS_MODE_64

import idautils
import idc
import idaapi
import ida_bytes
import ida_allins
import ida_kernwin

from time import time

class Emulator:
    def __init__(self):
        emu = Uc(UC_ARCH_X86, UC_MODE_64)

        # map in the required sections
        # .text and .data should be the only required ones
        # TODO: map even less data
        now = time()
        for s in idautils.Segments():
            start = idc.get_segm_start(s)
            end = idc.get_segm_end(s)
            name = idc.get_segm_name(s)
            # end must be rounded up to the nearest page
            if end % 0x1000 != 0:
                end += 0x1000 - end % 0x1000
            if name == ".text" or name == ".data":
                print(f"adding segment {name} from {hex(start)} to {hex(end)}")
                # these should not be written to so i can ensure that i can reuse the same emulator instance
                emu.mem_map(start, end - start, UC_PROT_EXEC | UC_PROT_READ)
                seg_bytes = ida_bytes.get_bytes(start, end - start)
                #print(type(seg_bytes))
                emu.mem_write(start, seg_bytes)
                #print("added")
                # for some reason, there are two .data segments
                # only need the first one, so i'll force it to stop after it
                if name == ".data":
                    break
        # done for now
        self.emu = emu
        print(f"added segments in {time() - now}")

    def emu_start(self, start, end, max_insns):
        self.emu.emu_start(start, end, count=max_insns)

    def run(self, start, end):
        pc = start
        reached_jmp = False
        for _ in range(100):
            self.emu_start(pc, -1, 1)
            pc = self.reg_read(UC_X86_REG_RIP)
            if pc == end:
                self.emu_start(pc, -1, 1)
                pc = self.reg_read(UC_X86_REG_RIP)
                reached_jmp = True
                break
        if not reached_jmp:
            print("didn't reach the end of the block in 100 instructions. this should not happen at all")
            return
        return pc

    def reset(self):
        # i don't want to unmap everything, since it's possible to reuse the current mappings
        # so clearing every register it is
        # some registers can't be run in this loop since they're supposed to take multiple arguments
        # it should still be fine though
        EXCLUDED_REGISTERS = {
            UC_X86_REG_IDTR,
            UC_X86_REG_GDTR,
            UC_X86_REG_LDTR,
            UC_X86_REG_TR,
            UC_X86_REG_MSR
        }
        for x in range(x86_const.UC_X86_REG_FP0, x86_const.UC_X86_REG_FP0 + 8):
            EXCLUDED_REGISTERS.add(x)
        for x in range(x86_const.UC_X86_REG_XMM0, x86_const.UC_X86_REG_XMM0 + 8):
            EXCLUDED_REGISTERS.add(x)
        for x in range(x86_const.UC_X86_REG_YMM0, x86_const.UC_X86_REG_YMM0 + 16):
            EXCLUDED_REGISTERS.add(x)
        
        for x in range(UC_X86_REG_INVALID + 1, UC_X86_REG_ENDING):
            if x in EXCLUDED_REGISTERS:
                continue
            self.reg_write(x, 0)

        # TEMPORARY
        #self.reg_write(UC_X86_REG_RBX, 2)
    
    def reg_read(self, reg):
        return self.emu.reg_read(reg)
    def reg_write(self, reg, val):
        self.emu.reg_write(reg, val)

    def mem_write(self, addr, data):
        self.emu.mem_write(addr, data)

def get_ida_insns(start, end):
    insns = []
    for ea in idautils.Heads(start, end):
        insn = idaapi.insn_t()
        idaapi.decode_insn(insn, ea)
        #print(insn.itype)
        insns.append(insn)
    return insns

def main():
    emu = Emulator()
    #emu.reg_write(UC_X86_REG_RAX, 0x11223344)
    #print(hex(emu.reg_read(UC_X86_REG_RAX)))
    if idc.read_selection_start() == idc.BADADDR:
        print("nothing is selected...")
        return
    block_start = idc.read_selection_start()
    block_end = idc.read_selection_end()
    print(f"analyzing from {hex(block_start)} to {hex(block_end)}")
    insns = get_ida_insns(block_start, block_end)
    
    # get and save any special instructions between the add and the jmp
    # an example of this happening is at 0x1800F9F6B in 1.5-dev's UnityPlayer.dll
    # these instructions should be able to be safely moved
    # also, they are useless in the context of figuring out the branch targets
    add_idx = -1
    for i in range(len(insns) - 1, -1, -1):
        if i == len(insns) - 1:
            if insns[i].itype != ida_allins.NN_jmpni:
                print(f"wtf, last instruction is not a register jmp")
                return
        else:
            if insns[i].itype == ida_allins.NN_add:
                add_idx = i
                break
    if add_idx == -1:
        print("couldn't find the add...")
        return
    extra_insns = insns[add_idx + 1:-1]
    extra_insn_bytes = bytes()
    for x in extra_insns:
        extra_insn_bytes += ida_bytes.get_bytes(x.ea, x.size)
    print(f"got {len(extra_insns)} extra instructions ({len(extra_insn_bytes)} bytes)")
    if len(extra_insns) > 0:
        extra_insn_start = extra_insns[0].ea
        extra_insn_end = extra_insn_start + len(extra_insn_bytes)
        print(f"nopping emulator memory from {hex(extra_insn_start)} to {hex(extra_insn_end)}")
        patch = b'\x90' * len(extra_insn_bytes)
        emu.mem_write(extra_insn_start, patch)
    
    # emulate!
    CASES_TO_TEST = [
        ("none", "none", 0x0000),
        ("eflags", "carry", 0x0001),
        ("eflags", "zero", 0x0040),
        ("eflags", "overflow", 0x0800),
        
        ("register", "dl", UC_X86_REG_DL),
        ("register", "r8b", UC_X86_REG_R8B),
        ("register", "cl", UC_X86_REG_CL),
        ("register", "r9b", UC_X86_REG_R9B),
        ("register", "al", UC_X86_REG_AL),
        ("register", "bl", UC_X86_REG_BL),
        ("register", "r10b", UC_X86_REG_R10B),
    ]
    results = [None] * (len(CASES_TO_TEST) - 1)
    no_flag_pc = 0
    branch_pc = 0
    try:
        start = time()
        for i in range(len(CASES_TO_TEST)):
            x = CASES_TO_TEST[i]
            emu.reset()
            if x[0] == "eflags":
                emu.reg_write(UC_X86_REG_EFLAGS, x[2])
            elif x[0] == "register":
                emu.reg_write(x[2], 1)
            pc = emu.run(block_start, insns[-1].ea)
            if x[0] == "eflags":
                print(f"jumped to {hex(pc)} with flag {x[1]} set")
            elif x[0] == "register":
                print(f"jumped to {hex(pc)} with register {x[1]} set")
            elif x[0] == "none":
                print(f"jumped to {hex(pc)} with nothing set")
            else:
                print("what")
                return
            if x[0] == "none":
                no_flag_pc = pc
            else:
                results[i - 1] = pc != no_flag_pc
                if pc != no_flag_pc:
                    branch_pc = pc
        print(f"emulated stuff in {time() - start}")
        print(results)
    except UcError as e:
        print(f"got error {e} at {hex(emu.reg_read(UC_X86_REG_RIP))}")
        return
    
    print(f"targets are {hex(no_flag_pc)} and {hex(branch_pc)}")

    # find an appropriate branch instruction to replace the flags being checked
    # if all registers being 0 already satisfies the constraints, the target addresses will be swapped
    jump_insn = ""
    reg_to_test = ""
    swap_targets = None
    if results == [None, None, None]:
        print("something broke")
        return
    elif results == [False, True, False, False, False, False, False, False, False, False]:
        # branch if ZF=1
        jump_insn = "jz"
        swap_targets = False
    elif results == [True, False, False, False, False, False, False, False, False, False]:
        # branch if CF=1
        jump_insn = "jc"
        swap_targets = False
    elif results == [True, True, False, False, False, False, False, False, False, False]:
        # branch if CF=0 and ZF=0
        jump_insn = "jnbe"
        swap_targets = True
        #swap_targets = False
    elif results == [False, False, True, False, False, False, False, False, False, False]:
        # branch if OF=1
        jump_insn = "jo"
        swap_targets = False
    elif results == [False, False, False, True, False, False, False, False, False, False]:
        # branch if DL=1
        reg_to_test = "dl"
        swap_targets = False
    elif results == [False, False, False, False, True, False, False, False, False, False]:
        # branch if R8B=1
        reg_to_test = "r8b"
        swap_targets = False
    elif results == [False, False, False, False, False, True, False, False, False, False]:
        # branch if CL=1
        reg_to_test = "cl"
        swap_targets = False
    elif results == [False, False, False, False, False, False, True, False, False, False]:
        # branch if R9B=1
        reg_to_test = "r9b"
        swap_targets = False
    elif results == [False, False, False, False, False, False, False, True, False, False]:
        # branch if AL=1
        reg_to_test = "al"
        swap_targets = False
    elif results == [False, False, False, False, False, False, False, False, True, False]:
        # branch if BL=1
        reg_to_test = "bl"
        swap_targets = False
    elif results == [False, False, False, False, False, False, False, False, False, True]:
        # branch if R10B
        reg_to_test = "r10b"
        swap_targets = False
    else:
        print("unhandled results table")
        return
    if swap_targets == None:
        print("i am retarded")
        return

    # start assembling
    # probably not the best way to do this
    jump_targets = [None, None]
    if swap_targets:
        jump_targets = [
            branch_pc,
            no_flag_pc
        ]
    else:
        jump_targets = [
            no_flag_pc,
            branch_pc
        ]
    
    ks = Ks(KS_ARCH_X86, KS_MODE_64)
    patch = extra_insn_bytes

    # TEMPORARY
    #patch += b"\x90" * 7

    if reg_to_test == "":
        # branch based on flag
        to_assemble = f"{jump_insn} {hex(jump_targets[1] - (block_start + len(patch)))}"
        patch += bytes(ks.asm(to_assemble)[0])
        to_assemble = f"jmp {hex(jump_targets[0] - (block_start + len(patch)))}"
        patch += bytes(ks.asm(to_assemble)[0])
    else:
        # branch based on register being non-zero
        to_assemble = f"test {reg_to_test},{reg_to_test}"
        patch += bytes(ks.asm(to_assemble)[0])
        to_assemble = f"jnz {hex(jump_targets[1] - (block_start + len(patch)))}"
        patch += bytes(ks.asm(to_assemble)[0])
        to_assemble = f"jmp {hex(jump_targets[0] - (block_start + len(patch)))}"
        patch += bytes(ks.asm(to_assemble)[0])

    if len(patch) > block_end - block_start:
        print("generated patch is too big")
        return

    # pad the patch with int3s
    for _ in range(block_end - block_start - len(patch)):
        patch += b'\xCC'
    print(f"patch: {patch}")

    # go!
    ida_bytes.patch_bytes(block_start, patch)

    # clean stuff up a little
    for x in range(block_start, block_end):
        idc.create_insn(x)

#main()

ida_kernwin.add_hotkey("SHIFT-D", main)