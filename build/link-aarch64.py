#!/usr/bin/env python3

import clang_link
import sys

completed_process = clang_link.link("aarch64-unknown-linux-gnu")
sys.exit(completed_process.returncode)
