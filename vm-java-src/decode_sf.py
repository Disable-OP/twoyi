#!/usr/bin/env python3
"""Decodes StringFog (Vigenere-XOR) strings from jadx-decompiled Java sources."""
import os, re, sys

TAR = {'LF_NORMAL':0x30,'LF_LINK':0x31,'LF_SYMLINK':0x32,'LF_CHR':0x33,'LF_BLK':0x34,'LF_DIR':0x35,'LF_FIFO':0x36,'LF_CONTIG':0x37,'LF_GNUTYPE_LONGNAME':0x4C,'LF_GNUTYPE_LONGLINK':0x4B,'LF_MULTIVOLUME':0x4D,'LF_GNUTYPE_SPARSE':0x53,'LF_PAX_EXTENDED_HEADER_LC':0x78,'LF_PAX_GLOBAL_EXTENDED_HEADER':0x67,'LF_PAX_EXTENDED_HEADER_UC':0x58}
CP = {'CP_Class':7,'CP_Fieldref':9,'CP_Methodref':10,'CP_InterfaceMethodref':11,'CP_NameAndType':12,'CP_String':8}
BYTE = {'Byte.MAX_VALUE':127,'Byte.MIN_VALUE':-128}

def resolve_token(tok):
    tok = tok.strip()
    if not tok: raise ValueError()
    if tok in TAR: return TAR[tok]
    if tok in CP: return CP[tok]
    if tok in BYTE: return BYTE[tok]
    if tok.startswith('TarConstants.'): return TAR[tok.split('.',1)[1]]
    if tok.startswith('ConstantPoolEntry.'): return CP[tok.split('.',1)[1]]
    if tok.startswith('Byte.'): return BYTE[tok]
    return int(tok)

def to_byte(v):
    v &= 0xFF
    if v >= 0x80: v -= 0x100
    return v

def decode(cipher, key):
    out = bytearray(len(cipher))
    for i, c in enumerate(cipher):
        k = key[i % len(key)]
        out[i] = (to_byte(c) ^ to_byte(k)) & 0xFF
    try: return out.decode('utf-8')
    except: return out.decode('latin-1')

PATTERN = re.compile(r'(?:x5\.WWWWWWWW|StringFog)\.m(?:17835|5049)WWWWWWWW\(new byte\[\]\{([^}]+)\},\s*new byte\[\]\{([^}]+)\}\)')

def scan_file(path):
    with open(path, encoding='utf-8', errors='ignore') as f: text = f.read()
    results = []
    for m in PATTERN.finditer(text):
        try:
            cipher = [resolve_token(t) for t in m.group(1).split(',') if t.strip()]
            key = [resolve_token(t) for t in m.group(2).split(',') if t.strip()]
            d = decode(cipher, key)
            line = text.count('\n', 0, m.start()) + 1
            results.append((line, d))
        except: pass
    return results

def main():
    if len(sys.argv) < 2:
        print("usage: decode_sf.py <dir-or-file> [filter_regex]")
        sys.exit(1)
    target = sys.argv[1]
    filt = re.compile(sys.argv[2]) if len(sys.argv) > 2 else None
    files = []
    if os.path.isdir(target):
        for root, _, fs in os.walk(target):
            for f in fs:
                if f.endswith('.java'): files.append(os.path.join(root, f))
    else: files = [target]
    for fp in sorted(files):
        for line, decoded in scan_file(fp):
            if filt and not filt.search(decoded): continue
            rel = os.path.relpath(fp, target) if os.path.isdir(target) else os.path.basename(fp)
            print(f"{rel}:{line}: {decoded!r}")

if __name__ == '__main__':
    main()
