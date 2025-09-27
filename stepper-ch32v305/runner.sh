#!/bin/bash

set -e

#"$@"
# upload with probe-rs so that we can preverify and verify afterwards
#probe-rs download --preverify --verify --chip ch32v305 "$@"
wlink flash --enable-sdi-print --watch-serial $@
#enable sdi-print for debug
#wlink sdi-print enable
#open serial, and have fun!
socat /dev/ttyACM0,rawer,b115200 STDOUT | defmt-print --show-skipped-frames  -v -e "$@"