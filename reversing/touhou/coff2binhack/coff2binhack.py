import sys
import json5 # pip install json5
import binascii
import itertools
import struct
import coff
from coff import Coff # pip install coff
from coff import IMAGE_SCN_MEM_EXECUTE, IMAGE_SCN_MEM_READ, IMAGE_SCN_MEM_WRITE, IMAGE_SCN_CNT_UNINITIALIZED_DATA, IMAGE_SCN_ALIGN_MASK, IMAGE_SCN_LNK_COMDAT

# Common functions provided by thcrap
COMMON_IMPORTS = {
    # Allocation functions
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

    # Memory functions
    "_memcpy": "th_memcpy",
    "_memmove": "th_memmove",
    "_memcmp": "th_memcmp",
    "_memset": "th_memset",
    "_memccpy": "th_memccpy",
    "_memchr": "th_memchr",
    "_strdup": "th_strdup",
    "_strndup": "th_strndup",
    "_strdup_size": "th_strdup_size",

    # String functions
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
    "_strchr": "th_strchr",
    "_strrchr": "th_strrchr",
    "_strstr": "th_strstr",
    "__strrev": "th_strrev",

    # Formatting functions
    "_sprintf": "th_sprintf",
    "_vsprintf": "th_vsprintf",
    "_snprintf": "th_snprintf",
    "_vsnprintf": "th_vsnprintf",
    "_sscanf": "th_sscanf",
    "_vsscanf": "th_vsscanf",
    "_strftime": "th_strftime",

    # Math functions
    "_fabsf": "th_fabsf",
    "_fabs": "th_fabs",
    "_fmodf": "th_fmodf",
    "_fmod": "th_fmod",
    "_remainderf": "th_remainderf",
    "_remainder": "th_remainder",
    "_remquof": "th_remquof",
    "_remquo": "th_remquo",
    "_fmaf": "th_fmaf",
    "_fma": "th_fma",
    "_fmaxf": "th_fmaxf",
    "_fmax": "th_fmax",
    "_fminf": "th_fminf",
    "_fmin": "th_fmin",
    "_fdimf": "th_fdimf",
    "_fdim": "th_fdim",
    "_expf": "th_expf",
    "_exp": "th_exp",
    "_logf": "th_logf",
    "_log": "th_log",
    "_log10f": "th_log10f",
    "_log10": "th_log10",
    "_log2f": "th_log2f",
    "_log2": "th_log2",
    "_powf": "th_powf",
    "_pow": "th_pow",
    "_sqrtf": "th_sqrtf",
    "_sqrt": "th_sqrt",
    "_hypotf": "th_hypotf",
    "_hypot": "th_hypot",
    "_sinf": "th_sinf",
    "_sin": "th_sin",
    "_cosf": "th_cosf",
    "_cos": "th_cos",
    "_tanf": "th_tanf",
    "_tan": "th_tan",
    "_asinf": "th_asinf",
    "_asin": "th_asin",
    "_acosf": "th_acosf",
    "_acos": "th_acos",
    "_atanf": "th_atanf",
    "_atan": "th_atan",
    "_atan2f": "th_atan2f",
    "_atan2": "th_atan2",
    "_ceilf": "th_ceilf",
    "_ceil": "th_ceil",
    "_floorf": "th_floorf",
    "_floor": "th_floor",
    "_truncf": "th_truncf",
    "_trunc": "th_trunc",
    "_roundf": "th_roundf",
    "_round": "th_round",
    "_nearbyintf": "th_nearbyintf",
    "_nearbyint": "th_nearbyint",

    # Compiler intrinsics
    "__ftol": "th_ftol",
    "__ftol2": "th_ftol2",
    "__ftol2_sse": "th_ftol2_sse",
    "__CIfmod": "th_CIfmod",
    "__CIexp": "th_CIexp",
    "___libm_sse2_exp": "th_exp_sse2",
    "___libm_sse2_expf": "th_expf_sse2",
    "__CIlog": "th_CIlog",
    "___libm_sse2_log": "th_log_sse2",
    "___libm_sse2_logf": "th_logf_sse2",
    "__CIlog10": "th_CIlog10",
    "___libm_sse2_log10": "th_log10_sse2",
    "___libm_sse2_log10f": "th_log10f_sse2",
    "__CIpow": "th_CIpow",
    "___libm_sse2_pow": "th_pow_sse2",
    "___libm_sse2_powf": "th_powf_sse2",
    "__CIsqrt": "th_CIsqrt",
    "__CIsin": "th_CIsin",
    "___libm_sse2_sin": "th_sin_sse2",
    "___libm_sse2_sinf": "th_sinf_sse2",
    "__CIcos": "th_CIcos",
    "___libm_sse2_cos": "th_cos_sse2",
    "___libm_sse2_cosf": "th_cosf_sse2",
    "__CItan": "th_CItan",
    "___libm_sse2_tan": "th_tan_sse2",
    "___libm_sse2_tanf": "th_tanf_sse2",
    "__CIasin": "th_CIasin",
    "___libm_sse2_asin": "th_asin_sse2",
    "___libm_sse2_asinf": "th_asinf_sse2",
    "__CIacos": "th_CIacos",
    "___libm_sse2_acos": "th_acos_sse2",
    "___libm_sse2_acosf": "th_acosf_sse2",
    "__CIatan": "th_CIatan",
    "___libm_sse2_atan": "th_atan_sse2",
    "___libm_sse2_atanf": "th_atanf_sse2",
    "__CIatan2": "th_CIatan2",
    "___libm_sse2_atan2": "th_atan2_sse2",
    "__allmul": "th_allmul",
    "__alldiv": "th_alldiv",
    "__allrem": "th_allrem",
    "__alldvrm": "th_alldvrm",
    "__aulldiv": "th_aulldiv",
    "__aullrem": "th_aullrem",
    "__aulldvrm": "th_aulldvrm",
    "__allshl": "th_allshl",
    "__allshr": "th_allshr",
    "__aullshr": "th_aullshr",

    # Utility functions
    "_qsort": "th_qsort",
    "_bsearch": "th_bsearch",
    "_rand_s": "th_rand_s",

    # Windows functions (usually these get imported from DLLs but it doesn't hurt to add here)
    "_Sleep": "th_Sleep",
    "_GetLastError": "th_GetLastError",
    "_GetProcAddress": "th_GetProcAddress",
    "_GetModuleHandleA": "th_GetModuleHandleA",
    "_GetModuleHandleW": "th_GetModuleHandleW",
    "_QueryPerformanceCounter": "th_QueryPerformanceCounter",
    "_QueryPerformanceFrequency": "th_QueryPerformanceFrequency",
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
        self.codecaves = data.get("codecaves", {})

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
            if section.offrel != 0 and (section.flags & (coff.IMAGE_SCN_MEM_DISCARDABLE | coff.IMAGE_SCN_LNK_REMOVE)) == 0:
                curreloff = section.offrel
                for i in range(section.numrels):
                    rel = coff.CoffReloc(data,curreloff,basesymoff, stroff,strend)
                    rel.symidx = struct.unpack('<L', data[(curreloff+4):(curreloff+8)])[0]
                    if self._Coff__header.id == 0x8664:
                        if rel.type >= coff.IMAGE_REL_AMD64_REL32  and rel.type <= coff.IMAGE_REL_AMD64_REL32_5:
                            rel.size = 4
                            self._Coff__relocs[seckey].append(rel)
                        else:
                            raise Exception(f"Unhandled relocation type {rel.type}")
                    elif self._Coff__header.id == 0x14c:
                        if rel.type == coff.IMAGE_REL_I386_DIR32  or rel.type == coff.IMAGE_REL_I386_DIR32NB  or rel.type == coff.IMAGE_REL_I386_REL32 :
                            self._Coff__relocs[seckey].append(rel)
                            rel.size = 4
                        else:
                            raise Exception(f"Unhandled relocation type {rel.type}")
                    curreloff += rel.get_size()
        return
    Coff._Coff__parse_reloc = __parse_reloc
    obj = Coff(config.input)

    options = config.options.copy()
    codecaves = config.codecaves.copy()
    section_to_cave = dict() # (section number, (codecave/option string, offset))
    comdat_pool = str()
    const_count = 0
    xcu_id = None
    xtx_id = None
    for i, section in enumerate(obj.sections):
        if section.size == 0 or section.name.startswith("/") or section.name in [".drectve", ".llvm_addrsig", ".debug$S", ".debug$T"]:
            continue

        if section.name == ".CRT$XCU":
            xcu_id = i
        elif section.name == ".CRT$XTX":
            xtx_id = i

        prot = str()
        if section.flags & IMAGE_SCN_MEM_READ:
            prot += "r"
        if section.flags & IMAGE_SCN_MEM_WRITE:
            prot += "w"
        if section.flags & IMAGE_SCN_MEM_EXECUTE:
            prot += "x"

        if section.flags & IMAGE_SCN_LNK_COMDAT and not (section.flags & IMAGE_SCN_MEM_EXECUTE) and i in obj.symtables and len(obj.symtables[i]) == 1:
            # Constant deduplication section
            # TODO: try to decode floats in a way that's roundtrippable
            section_to_cave[i] = (f"option:{config.prefix}_const_{const_count}", 0)

            raw_string = raw_obj[section.offdata:(section.offdata + section.size)]
            option_type = "c"
            option_data = binascii.hexlify(raw_string).decode()

            if obj.symtables[i][0].name.startswith("??_C"):
                # Unfortunately, the string symbol name doesn't specify what encoding it is
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

            if option_type == "c":
                cur_size = len(comdat_pool) // 2
                alignment = 0
                if section.flags & IMAGE_SCN_ALIGN_MASK:
                    alignment = 1 << ((section.flags & IMAGE_SCN_ALIGN_MASK) >> 20) - 1
                padding = 0
                if alignment != 0 and cur_size % alignment:
                    padding = alignment - (cur_size % alignment)
                section_to_cave[i] = (f"codecave:{config.prefix}_comdat_pool", cur_size + padding)
                comdat_pool += "00" * padding
                comdat_pool += binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
            else:
                options[f"{config.prefix}_const_{const_count}"] = {
                    "type": option_type,
                    "val": option_data,
                }
                const_count += 1
        elif config.prefix + section.name + str(i) in codecaves:
            if "code" in codecaves[config.prefix + section.name]:
                section_to_cave[i] = (f"codecave:{config.prefix}{section.name}{i}", len(codecaves[config.prefix + section.name + str(i)]["code"]) // 2)
                codecaves[config.prefix + section.name + str(i)]["code"] += binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
            else:
                section_to_cave[i] = (f"codecave:{config.prefix}{section.name}{i}", codecaves[config.prefix + section.name + str(i)]["size"])
                codecaves[config.prefix + section.name + str(i)]["size"] += section.size
        else:
            #print(section, obj.relocs[i])
            if section.flags & IMAGE_SCN_CNT_UNINITIALIZED_DATA:
                codecaves[config.prefix + section.name + str(i)] = {
                    "access": prot,
                    "size": section.size
                }
            else:
                codecaves[config.prefix + section.name + str(i)] = {
                    "access": prot,
                    "code": binascii.hexlify(raw_obj[section.offdata:(section.offdata + section.size)]).decode()
                }
            section_to_cave[i] = (f"codecave:{config.prefix}{section.name}{i}", 0)
            config.externs[section.name] = Extern(f"codecave:{config.prefix}{section.name}{i}", 0)
    
    if len(comdat_pool) > 0:
        codecaves[config.prefix + "_comdat_pool"] = {
            "access": "r",
            "code": comdat_pool
        }

    config.externs.update({k: Extern(v, 0) for k, v in COMMON_IMPORTS.items()})
    for sym in itertools.chain(*obj.symtables.values()):
        cave = section_to_cave[sym.sectnum - 1]
        config.externs[sym.name if not sym.name.startswith(".") else f"{sym.name}_sectref{sym.sectnum}"] = Extern(cave[0], cave[1] + sym.value)
    
    if config.options:
        opt_cave = str()
        opt_offset = 0
        for name, opt in config.options.items():
            match opt["type"][0]:
                case "s" | "w" | "c":
                    size = 4
                case "i" | "b" | "u" | "p" | "f":
                    size = int(opt["type"][1:]) // 8
                case _:
                    raise Exception(f"Unhandled option type {opt['type']}")
            opt_cave += f"<option:{name}>"
            config.externs[opt["symbol"]] = Extern(f"codecave:{config.prefix}_options", opt_offset)
            opt_offset += size
        codecaves[config.prefix + "_options"] = {
            "access": "r",
            "code": opt_cave
        }
    
    init_code = str()
    if ".CRT$XCU" in config.externs:
        # See call_statics.asm
        length = next(x for x in obj.sections if x.name == ".CRT$XCU").size // 4
        init_code += f"5331db0f1f8400000000000f1f440000ff149d<codecave:{config.prefix}.CRT$XCU{xcu_id}>4381fb{binascii.hexlify(length.to_bytes(4, 'little')).decode()}75f05b"
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
                "access": "rx",
                "code": import_code,
                "export": True,
            },
            config.prefix + "_dlls": {
                "access": "r",
                "code": dlls_cave,
            },
            config.prefix + "_import_names": {
                "access": "r",
                "code": import_names_cave,
            },
            config.prefix + "_imports": {
                "access": "rw",
                "size": import_count * 4,
            },
        })
    else:
        codecaves[config.prefix + "_patch_init"] = {
            "access": "rx",
            "code": init_code,
            "export": True,
        }
    
    if ".CRT$XTX" in config.externs:
        # See call_statics.asm
        length = next(x for x in obj.sections if x.name == ".CRT$XTX").size // 4
        exit_code = f"5331db0f1f8400000000000f1f440000ff149d<codecave:{config.prefix}.CRT$XTX{xtx_id}>4381fb{binascii.hexlify(length.to_bytes(4, 'little')).decode()}75f05bc3"
        codecaves[config.prefix + "_patch_exit"] = {
            "access": "rx",
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
                if reloc.name.startswith("."):
                    # Stupid symbol resolution hack zone
                    # Sometimes clang will generate a block of data in .rdata with the symbol named ".rdata", then try to refer to it using that name
                    sym = coff.CoffSymtable(raw_obj, obj._Coff__symoffset + 18 * reloc.symidx, obj._Coff__stroffset, obj._Coff__stroffset + obj._Coff__strsize)
                    offset = int.from_bytes(raw_obj[(section.offdata + reloc.vaddr):(section.offdata + reloc.vaddr + 4)], byteorder="little") + sym.value
                else:
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
                code = codecaves[config.prefix + section.name + str(seckey)]["code"]
                codecaves[config.prefix + section.name + str(seckey)]["code"] = code[:(reloc.vaddr * 2)] + replacement + code[(reloc.vaddr * 2 + 8):]
            else:
                print(config.externs)
                raise KeyError(f"Unhandled reloc {reloc}")

    # TODO: this is really jank and badly implemented
    def rewrite_obj_ref(code):
        obj_pos = code.find("obj:")
        while obj_pos != -1:
            SEPARATORS = [' ', ')', ']', '}', '>', '+']
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
        return code

    for binhack in config.binhacks.values():
        binhack["code"] = rewrite_obj_ref(binhack["code"])
    for codecave in config.codecaves.values():
        codecave["code"] = rewrite_obj_ref(codecave["code"])

    output_dict = {
        "options": options,
        "codecaves": codecaves,
        "binhacks": config.binhacks
    }
    with open(config.output, "w", encoding="utf-8") as output:
        json5.dump(output_dict, output, indent=4, ensure_ascii=False)
