import idaapi
import idautils
import ida_funcs
import ida_allins
import ida_bytes
import ida_ida
import ida_nalt
import idc

from miasm.core.bin_stream_ida import bin_stream_ida
from miasm.analysis.machine import Machine
from miasm.core.locationdb import LocationDB
from miasm.analysis.depgraph import DependencyGraph
from miasm.expression.expression import *

# TODO: Actual documentation
# I made this script as a mostly one-time use thing and didn't spend much time cleaning it up for public release
# Just make an issue if you actually want to use this script and I'll explain it :p


def decrypt(addr, s1, s2):
    rdata_start = idaapi.get_segm_by_name(".rdata").start_ea
    rdata_end = idaapi.get_segm_by_name(".rdata").end_ea
    if addr >= rdata_start and addr < rdata_end:
        input = idaapi.get_bytes(addr, s1 + s2)
        out = bytearray(b"\x00" * s2)
        for i in range(s2):
            out[i] = input[i % s1] ^ input[s1 + i]
        print(out)
    else:
        print(hex(rdata_start))
        print(hex(rdata_end))
        raise Exception("wrong arg dumbass")
    return out


def find_call(ir_arch, addr, func_addr):
    """
    Returns irb, index which call @func_addr
    """

    irbs = ir_arch.getby_offset(addr)
    out = set()
    for irb in irbs:
        if len(irb.irs) < 2:
            continue
        assignblk = irb.irs[-2]
        for dst, src in assignblk.iteritems():
            if not isinstance(src, ExprOp):
                continue
            if not src.op.startswith("call_func"):
                continue
            out.add((irb, len(irb.irs) - 2))
    assert len(out) == 1
    irb, index = list(out)[0]
    return irb, index


# Get the current function
# addr = idc.get_screen_ea()

# Init
machine = Machine("x86_64")
mn, dis_engine, lifter_model_call = (
    machine.mn,
    machine.dis_engine,
    machine.lifter_model_call,
)

bs = bin_stream_ida()
loc_db = LocationDB()

mdis = dis_engine(bs, loc_db=loc_db, dont_dis_nulstart_bloc=True)
lifter = lifter_model_call(loc_db)

# Populate symbols with ida names
for ad, name in idautils.Names():
    if name is None:
        continue
    loc_db.add_location(name, ad)

ir_cache = dict()


def miasm_solve_args(addr):
    global ir_cache
    func = ida_funcs.get_func(addr)
    asmcfg = mdis.dis_multiblock(func.start_ea)

    # Generate IR
    ircfg = None
    if func.start_ea in ir_cache:
        # print("cached")
        ircfg = ir_cache[func.start_ea]
    else:
        ircfg = lifter.new_ircfg_from_asmcfg(asmcfg)
        ir_cache[func.start_ea] = ircfg

    cur_block = None
    for loc_key in ircfg.getby_offset(addr):
        block = ircfg.get_block(loc_key)
        offset = ircfg.loc_db.get_location_offset(block.loc_key)
        if offset is not None:
            # Only one block non-generated
            assert cur_block is None
            cur_block = block
    assert cur_block is not None
    line_nb = None
    for line_nb, assignblk in enumerate(cur_block):
        if assignblk.instr.offset == addr:
            break
    assert line_nb is not None

    # print(cur_block)
    # print(line_nb)

    g_dep = DependencyGraph(ircfg, follow_call=False)
    graph = g_dep.get(
        cur_block.loc_key,
        [lifter.arch.regs.RCX, lifter.arch.regs.RDX, lifter.arch.regs.R8],
        line_nb,
        set([loc_db.get_offset_location(func.start_ea)]),
    )
    solutions = list(graph)
    assert len(solutions) == 1

    solution = solutions[0].emul(lifter)
    return (
        int(solution[ExprId("RCX", 64)]),
        int(solution[ExprId("RDX", 64)]),
        int(solution[ExprId("R8", 64)]),
    )


def ida_solve_arg(addr):
    args = idaapi.get_arg_addrs(addr)
    assert len(args) == 5
    arg_addr = args[-1]
    # print(hex(arg_addr))

    insn = idaapi.insn_t()
    idaapi.decode_insn(insn, arg_addr)
    # print(insn.itype)
    assert insn.itype == ida_allins.NN_mov
    assert insn.size == 8
    assert insn.ops[0].dtype == 2
    assert insn.ops[1].dtype == 2
    return insn.ops[1].value


resolved = {}
addrs = [
    0x18017E390,
    0x1801A2FA0,
    0x1801B9F10,
    0x18025C5D0,
    0x180274160,
    0x180287FA0,
    0x180298C30,
    0x1802A5C70,
    0x1802F7CE0,
    0x180330350,
    0x180341E70,
    0x18034E580,
    0x180395760,
    0x1803990C0,
    0x1803DCF70,
    0x180409F70,
    0x180415F80,
    0x1804537C0,
    0x180459ED0,
    0x1804610C0,
    0x18046AF00,
    0x18046C200,
]
text_start = idaapi.get_segm_by_name(".text").start_ea
text_end = idaapi.get_segm_by_name(".text").end_ea
for addr in addrs:
    # This is in a separate loop so it's possible to go through all of the decryption functions manually
    # IDA doesn't really want to auto detect function arguments without some manual intervention
    assert (
        idc.SetType(
            addr,
            "void __fastcall sub_18017E390(char *a1, char *a2, unsigned int a3, _DWORD *a4, int a5)",
        )
        == 1
    )
for addr in addrs:
    for ref in idautils.XrefsTo(addr):
        if ref.frm >= text_start and ref.frm < text_end:
            print(f"solving call at {hex(ref.frm)}")
            rcx, rdx, r8 = miasm_solve_args(ref.frm)
            stack_arg = ida_solve_arg(ref.frm)
            # decrypt(rcx, rdx, r8, stack_arg)
            # idc.set_cmt(ref.frm, decrypt(rdx, r8, stack_arg), 0)
            resolved[rcx] = bytes(decrypt(rdx, r8, stack_arg))
print(resolved)

for addr, data in resolved.items():
    ida_bytes.put_bytes(addr, data)
    wide = len(data) > 1 and data[1] == b"\x00"
    if wide:
        ty = "wchar_t"
    else:
        ty = "char"
    assert (
        idc.SetType(
            addr,
            f"const {ty} blah[]",
        )
        == 1
    )
    if wide:
        ida_ida.inf_set_strtype(ida_nalt.STRTYPE_C_16)
    else:
        ida_ida.inf_set_strtype(ida_nalt.STRTYPE_C)
    idc.create_strlit(addr, idc.BADADDR)
