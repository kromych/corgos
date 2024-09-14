#!/bin/sh

export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
clang --target=x86_64-unknown-linux-gnu $@
