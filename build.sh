#!/usr/bin/env bash
export RUSTFLAGS="-C target-cpu=native"
maturin build --no-sdist --manylinux off --interpreter python3 --release --strip
pip install target/wheels/*.whl -U --force-reinstall
