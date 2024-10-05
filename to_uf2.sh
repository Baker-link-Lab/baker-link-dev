#!/bin/bash
TRG=target/thumbv6m-none-eabi/release/baker-link
OUT=release/baker-link

rm $OUT
elf2uf2-rs $TRG $OUT

ls -l release