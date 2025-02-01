import os
import platform
import subprocess
import sys

def link(target):
    clang = '/opt/homebrew/opt/llvm/bin/clang' if platform.system() == "Darwin" else 'clang'
    result = subprocess.run([clang, f'--target={target}'] + sys.argv[1:], 
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    print(result.stdout)
    print(result.stderr, file=sys.stderr)

    return result
