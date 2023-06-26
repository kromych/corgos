#!/usr/bin/env python3
    
class VirtAddr:
    def __init__(self, va):
        # Page size 4KiB
        self.offset = va & 0xfff

        # Isn't yet aware of larger page sizes, or
        # fewer translation stages.
        self.lvl0 = (va >> 12) & 0x1ff
        self.lvl1 = (va >> 21) & 0x1ff
        self.lvl2 = (va >> 30) & 0x1ff
        self.lvl3 = (va >> 39) & 0x1ff

    def __repr__(self):
        return f"{type(self).__name__}["\
            f"L4:0x{self.lvl3:03x}, L3:0x{self.lvl2:03x}, "\
            f"L2:0x{self.lvl1:03x}, L1:0x{self.lvl0:03x}, "\
            f"offset:0x{self.offset:03x}]"

if __name__ == "__main__":
    import sys

    args = sys.argv[1:]
    if not args:
        print("This script breaks a virtual address (VA) down to its constituents.")
        print("Supply one integer for the VA, or a list of Python expressions each of which evalutes to an integer.")
        exit(1)

    for va in args:
        va = int(eval(va))
        va4 = VirtAddr(va)
        print(hex(va), va4)
    