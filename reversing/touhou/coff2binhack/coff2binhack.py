import sys
import json5 # pip install json5
import binascii
import coff
from coff import Coff # pip install coff
from coff import IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE

class Extern:
    def __init__(self, data: dict):
        self.addr = int(data["addr"][2:], 16)
        self.relative = data["addr"].lower().startswith("rx")
    
    def __str__(self) -> str:
        if self.relative:
            return f"Rx{self.addr:x}"
        else:
            return f"0x{self.addr:x}"

class Config:
    def __init__(self, data: dict):
        self.input = data["input"]
        self.output = data["output"]

        if data["externs"] is None:
            self.externs = {}
        else:
            self.externs = {x: Extern(y) for x, y in data["externs"].items()}

        if data["binhacks"] is None:
            self.binhacks = {}
        else:
            self.binhacks = data["binhacks"]

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} [input json]")
        sys.exit(1)
    
    with open(sys.argv[1], "r") as file:
        config = Config(json5.load(file))
    with open(config.input, "rb") as file:
        raw_obj = file.read()
    obj = Coff(config.input)

    codecaves = {}
    for seckey in obj.relocs.keys():
        # TODO: support bss
        section = obj.sections[seckey]
        prot = ""
        if section.flags & IMAGE_SCN_MEM_READ:
            prot += "r"
        if section.flags & IMAGE_SCN_MEM_WRITE:
            prot += "w"
        if section.flags & IMAGE_SCN_MEM_EXECUTE:
            prot += "x"

        # TODO: support more reloc types
        code = binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
        print(obj.relocs[seckey])
        relocs = sorted(obj.relocs[seckey], key=lambda rel: rel.vaddr, reverse=True)
        for reloc in relocs:
            if reloc.name in config.externs:
                assert reloc.size == 4
                match reloc.type:
                    case coff.IMAGE_REL_I386_DIR32:
                        replacement = f"<{config.externs[reloc.name]}>"
                    case coff.IMAGE_REL_I386_REL32:
                        replacement = f"[{config.externs[reloc.name]}]"
                    case _:
                        raise KeyError(f"Unhandled reloc type {hex(reloc.type)}")
                code = code[:(reloc.vaddr * 2)] + replacement + code[(reloc.vaddr * 2 + 8):]
            else:
                raise KeyError(f"Unhandled reloc {reloc}")

        codecaves[section.name] = {
            "prot": prot,
            "code": code
        }
    print(codecaves)

    output = {
        "codecaves": codecaves,
        "binhacks": config.binhacks
    }