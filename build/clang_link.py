import os
import platform
import subprocess
import sys

def link(target):
    if platform.system() == "Darwin":
        os.environ['PATH'] = "/opt/homebrew/opt/llvm/bin:" + os.environ['PATH']

    result = subprocess.run(['clang', f'--target={target}'] + sys.argv[1:], 
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    print(result.stdout)
    print(result.stderr, file=sys.stderr)

    return result
