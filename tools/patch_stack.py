"""Raises the main thread's stack reserve of a horizon-client.exe built with the fast profile.

The fast profile makes start_game's frame ~780 KB (release: ~143 KB) and the exe asks for 2 MB, so
the game overflows its stack on start. Usage: python patch_stack.py path\to\horizon-client.exe
"""
import struct, sys, shutil

path = sys.argv[1]
data = bytearray(open(path, 'rb').read())
pe = struct.unpack_from('<I', data, 0x3C)[0]
offset = pe + 24 + 72  # OptionalHeader.SizeOfStackReserve (PE32+)
before = struct.unpack_from('<Q', data, offset)[0]
if before >= 0x1000000:
    print('already', hex(before))
else:
    shutil.copyfile(path, path + '.bak-stack')
    struct.pack_into('<Q', data, offset, 0x1000000)
    open(path, 'wb').write(data)
    print(hex(before), '->', hex(0x1000000))
