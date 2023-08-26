import sys
import json5 # pip install json5
import binascii
import itertools
import coff
from coff import Coff # pip install coff
from coff import IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE, IMAGE_SCN_CNT_UNINITIALIZED_DATA, IMAGE_SCN_LNK_COMDAT
from types import MethodType

# Common functions provided by thcrap
COMMON_IMPORTS = {
    "_malloc": "th_malloc",
    "_calloc": "th_calloc",
    "_realloc": "th_realloc",
    "_free": "th_free",
    "__msize": "th_msize",
    "__expand": "th_expand",
    "__aligned_malloc": "th_aligned_malloc",
    "__aligned_realloc": "th_aligned_realloc",
    "__aligned_free": "th_aligned_free",
    "__aligned_msize": "th_aligned_msize",
    "_memcpy": "th_memcpy",
    "_memmove": "th_memmove",
    "_memcmp": "th_memcmp",
    "_memset": "th_memset",
    "_memccpy": "th_memccpy",
    "_strdup": "th_strdup",
    "_strndup": "th_strndup",
    "_strdup_size": "th_strdup_size",
    "_strcmp": "th_strcmp",
    "_strncmp": "th_strncmp",
    "_stricmp": "th_stricmp",
    "_strnicmp": "th_strnicmp",
    "_strcpy": "th_strcpy",
    "_strncpy": "th_strncpy",
    "_strcat": "th_strcat",
    "_strncat": "th_strncat",
    "_strlen": "th_strlen",
    "_strnlen_s": "th_strnlen_s",
    "_sprintf": "th_sprintf",
    "_snprintf": "th_snprintf",
    "_sscanf": "th_sscanf",
}

class Extern:
    def __init__(self, addr: str, offset: int):
        self.addr = addr
        self.offset = offset
    
    def __repr__(self):
        return f"{self.addr}+{hex(self.offset)}"

class Config:
    def __init__(self, data: dict):
        self.input = data["input"]
        self.output = data["output"]
        self.prefix = data["prefix"]
        self.externs = {x: Extern(y["addr"], y.get("offset", 0)) for x, y in data.get("externs", {}).items()}
        self.binhacks = data.get("binhacks", {})
        self.imports = data.get("imports", {})
        self.options = data.get("options", {})

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} [input json]")
        sys.exit(1)
    
    with open(sys.argv[1], "r", encoding="utf-8") as file:
        config = Config(json5.load(file))
    with open(config.input, "rb") as file:
        raw_obj = file.read()

    # We need to monkeypatch the COFF library to allow relocations in non-code sections
    def __parse_reloc(self,data):
        basesymoff = self._Coff__symoffset
        stroff = self._Coff__stroffset
        strend = (self._Coff__stroffset + self._Coff__strsize)
        self._Coff__relocs = dict()
        idx = 0
        for section in self.sections:
            idx += 1
            seckey = (idx - 1)
            self._Coff__relocs[seckey] = []
            if section.offrel != 0 and (section.flags & coff.IMAGE_SCN_LNK_COMDAT) == 0:
                curreloff = section.offrel
                for i in range(section.numrels):
                    rel = coff.CoffReloc(data,curreloff,basesymoff, stroff,strend)
                    if self._Coff__header.id == 0x8664:
                        if rel.type >= coff.IMAGE_REL_AMD64_REL32  and rel.type <= coff.IMAGE_REL_AMD64_REL32_5:
                            rel.size = 4
                            self.__relocs[seckey].append(rel)
                    elif self._Coff__header.id == 0x14c:
                        if rel.type == coff.IMAGE_REL_I386_DIR32  or rel.type == coff.IMAGE_REL_I386_DIR32NB  or rel.type == coff.IMAGE_REL_I386_REL32 :
                            self._Coff__relocs[seckey].append(rel)
                            rel.size = 4
                    curreloff += rel.get_size()
        return
    Coff._Coff__parse_reloc = __parse_reloc
    obj = Coff(config.input)

    options = config.options
    codecaves = {}
    section_to_cave = dict() # (section number, (codecave/option string, offset))
    const_count = 0
    for i, section in enumerate(obj.sections):
        if section.size == 0 or section.name.startswith("/") or section.name in [".drectve", ".llvm_addrsig"]:
            continue

        prot = str()
        if section.flags & IMAGE_SCN_MEM_READ:
            prot += "r"
        if section.flags & IMAGE_SCN_MEM_WRITE:
            prot += "w"
        if section.flags & IMAGE_SCN_MEM_EXECUTE:
            prot += "x"

        if section.flags & IMAGE_SCN_LNK_COMDAT and len(obj.symtables[i]) == 1 and obj.symtables[i][0].name.startswith("??_C"):
            # TODO: handle float constant deduplication sections too
            # String deduplication section
            # Unfortunately, the string symbol name doesn't specify what encoding it is
            section_to_cave[i] = (f"option:{config.prefix}_const_{const_count}", 0)

            raw_string = raw_obj[section.offdata:(section.offdata + section.size)]
            option_type = "c"
            option_data = binascii.hexlify(raw_string).decode()
            if len(raw_string) >= 1 and all(x > 0 and x < 0x80 for x in raw_string[:-1]) and raw_string[-1] == 0:
                # TODO: this won't work on utf-8 or shift-jis strings
                option_data = raw_string[:-1].decode(encoding="ascii")
                option_type = "s"
            elif len(raw_string) >= 2:
                try:
                    option_data = raw_string[:-2].decode(encoding="utf-16")
                    option_type = "w"
                except UnicodeError:
                    pass

            options[f"{config.prefix}_const_{const_count}"] = {
                "type": option_type,
                "val": option_data,
            }
            const_count += 1
        elif config.prefix + section.name in codecaves:
            section_to_cave[i] = (f"codecave:{config.prefix}{section.name}", len(codecaves[config.prefix + section.name]["code"]) // 2)
            codecaves[config.prefix + section.name]["code"] += binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
        else:
            #print(section, obj.relocs[i])
            if section.flags & IMAGE_SCN_CNT_UNINITIALIZED_DATA:
                codecaves[config.prefix + section.name] = {
                    "prot": prot,
                    "size": section.size
                }
            else:
                codecaves[config.prefix + section.name] = {
                    "prot": prot,
                    "code": binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
                }
            section_to_cave[i] = (f"codecave:{config.prefix}{section.name}", 0)
            config.externs[section.name] = Extern(f"codecave:{config.prefix}{section.name}", 0)

    config.externs.update({k: Extern(v, 0) for k, v in COMMON_IMPORTS.items()})
    for sym in itertools.chain(*obj.symtables.values()):
        cave = section_to_cave[sym.sectnum - 1]
        config.externs[sym.name] = Extern(cave[0], cave[1] + sym.value)
    
    init_code = str()
    if ".CRT$XCU" in config.externs:
        # See call_statics.asm
        length = next(x for x in obj.sections if x.name == ".CRT$XCU").size // 4
        init_code += f"5331db0f1f8400000000000f1f440000ff149d<codecave:{config.prefix}.CRT$XCU>4381fb{binascii.hexlify(length.to_bytes(4, 'little')).decode()}75f05b"
    if "_coff2binhack_init" in config.externs:
        extern = config.externs["_coff2binhack_init"]
        init_code += f"e8([{extern.addr}]+{hex(extern.offset)})"
    init_code += "c3"

    if len(config.imports) != 0:
        # See load_imports.asm
        import_code = f"555357565068<option:kernel32_dll_str>e8[th_GetModuleHandleA]68<option:LoadLibraryA_str>50e8[th_GetProcAddress]89042431edbb<codecave:{config.prefix}_dlls>8b35<codecave:{config.prefix}_dlls>660f1f44000056e8[th_GetModuleHandleA]89c785c0750b56ff54240489c785c0743c8b730439f5741f6690ff34ad<codecave:{config.prefix}_import_names>57e8[th_GetProcAddress]85c0742b8904ad<codecave:{config.prefix}_imports>4539ee75e38b730883c30885f675b983c4045e5f5b5de92c000000505668<option:dll_load_failed_str>eb0d50ff34ad<codecave:{config.prefix}_import_names>68<option:import_failed_str>e8[th_GetLastError]894424086a1068<option:{config.prefix}_name_str>e8[log_mboxf]cc"
        if init_code != "c3":
            import_code += init_code
        else:
            import_code = import_code.replace("e92c000000", "c30f1f4000")

        dlls_cave = str()
        import_names_cave = str()
        import_count = 0
        add_opt_str = lambda name, val: options.__setitem__(f"{name}_str", { "type": "s", "val": val })
        for dll, imports in config.imports.items():
            add_opt_str(f"{dll.replace('.', '_')}", dll)
            dlls_cave += f"<option:{dll.replace('.', '_')}_str>"
            for imp, imp_data in imports.items():
                add_opt_str(f"{imp}", imp)
                import_names_cave += f"<option:{imp}_str>"
                config.externs["__imp_" + imp_data.get("alias", imp)] = Extern(f"codecave:{config.prefix}_imports", import_count * 4)
                import_count += 1
            dlls_cave += binascii.hexlify(import_count.to_bytes(4, "little")).decode()
        dlls_cave += "0" * 8
        add_opt_str(f"{config.prefix}_name", config.prefix)
        add_opt_str("kernel32_dll", "kernel32.dll")
        add_opt_str("LoadLibraryA", "LoadLibraryA")
        add_opt_str("dll_load_failed", "Failed to load DLL %s (code: 0x%p)")
        add_opt_str("import_failed", "Failed to load import %s (code: 0x%p)")
        
        codecaves.update({
            config.prefix + "_patch_init": {
                "prot": "rx",
                "code": import_code,
                "export": True,
            },
            config.prefix + "_dlls": {
                "prot": "r",
                "code": dlls_cave,
            },
            config.prefix + "_import_names": {
                "prot": "r",
                "code": import_names_cave,
            },
            config.prefix + "_imports": {
                "prot": "rw",
                "size": import_count * 4,
            },
        })
    else:
        codecaves[config.prefix + "_patch_init"] = {
            "prot": "rx",
            "code": init_code,
            "export": True,
        }
    
    if ".CRT$XTX" in config.externs:
        # See call_statics.asm
        length = next(x for x in obj.sections if x.name == ".CRT$XTX").size // 4
        exit_code = f"5331db0f1f8400000000000f1f440000ff149d<codecave:{config.prefix}.CRT$XTX>4381fb{binascii.hexlify(length.to_bytes(4, 'little')).decode()}75f05bc3"
        codecaves[config.prefix + "_patch_exit"] = {
            "prot": "rx",
            "code": exit_code,
            "export": True,
        }

    for seckey in obj.relocs.keys():
        section = obj.sections[seckey]
        relocs = sorted(obj.relocs[seckey], key=lambda rel: rel.vaddr, reverse=True)
        for reloc in relocs:
            if reloc.name in config.externs:
                extern = config.externs[reloc.name]
                assert reloc.size == 4
                offset = int.from_bytes(raw_obj[(section.offdata + reloc.vaddr):(section.offdata + reloc.vaddr + 4)], byteorder="little") + extern.offset
                match reloc.type:
                    case coff.IMAGE_REL_I386_DIR32:
                        if offset == 0:
                            replacement = f"<{extern.addr}>"
                        else:
                            replacement = f"(<{extern.addr}>+{hex(offset)})"
                    case coff.IMAGE_REL_I386_REL32:
                        if offset == 0:
                            replacement = f"[{extern.addr}]"
                        else:
                            replacement = f"([{extern.addr}]+{hex(offset)})"
                    case _:
                        raise KeyError(f"Unhandled reloc type {hex(reloc.type)}")
                code = codecaves[config.prefix + section.name]["code"]
                codecaves[config.prefix + section.name]["code"] = code[:(reloc.vaddr * 2)] + replacement + code[(reloc.vaddr * 2 + 8):]
            else:
                raise KeyError(f"Unhandled reloc {reloc}")

    # TODO: this is really jank and badly implemented
    for binhack in config.binhacks.values():
        code = binhack["code"]
        obj_pos = code.find("obj:")
        while obj_pos != -1:
            SEPARATORS = [' ', ')', ']', '}', '+']
            obj_end = obj_pos + len("obj:")
            while not code[obj_end] in SEPARATORS:
                obj_end += 1

            # TODO: figure out how to implement this with the newer codecave reference syntax
            extern = config.externs[code[(obj_pos + len("obj:")):obj_end]]
            replacement = extern.addr
            if extern.offset != 0:
                replacement += f"+{hex(extern.offset)}"
            code = code[:obj_pos] + replacement + code[obj_end:]
            
            obj_pos = code.find("obj:")
        binhack["code"] = code

    output_dict = {
        "options": options,
        "codecaves": codecaves,
        "binhacks": config.binhacks
    }
    with open(config.output, "w", encoding="utf-8") as output:
        json5.dump(output_dict, output, indent=4, ensure_ascii=False)
