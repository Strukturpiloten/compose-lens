#!/usr/bin/env python3
"""Thin compatibility entry point for the immutable shared validation planner."""

import runpy
from pathlib import Path

if __name__ == "__main__":
    runpy.run_path(str(Path(__file__).with_name("shared-ci.py")), run_name="__main__")
