#!/bin/sh

export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
clang --target=aarch64-unknown-linux-gnu $@
