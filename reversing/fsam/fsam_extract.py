# Extracts the binaries dropped by the main "sam" binary
# It then runs "wd" after this

import os
import idc
import ida_bytes
import ida_loader

base = idc.get_name_ea_simple("files_to_extract")
outdir = os.path.dirname(ida_loader.get_path(ida_loader.PATH_TYPE_IDB))

for i in range(10):
    ea = base + i * 0x20

    path_ea = ida_bytes.get_qword(ea)
    data_ea = ida_bytes.get_qword(ea + 0x8)
    length = ida_bytes.get_qword(ea + 0x10)

    path = ida_bytes.get_strlit_contents(path_ea, -1, 0).decode()
    data = ida_bytes.get_bytes(data_ea, length)

    with open(os.path.join(outdir, os.path.basename(path)), "wb") as f:
        f.write(data)

    print(f"extracted {path}")
