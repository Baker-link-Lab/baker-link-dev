#!/bin/bash
TRG=target/thumbv6m-none-eabi/release/baker-link-dev
OUT=release/baker-link-dev

rm $OUT
elf2uf2-rs $TRG $OUT

ls -l release