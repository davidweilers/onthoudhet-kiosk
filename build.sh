#!/bin/bash

cargo build --target aarch64-unknown-linux-gnu -r

scp target/aarch64-unknown-linux-gnu/release/onthoudhet-kiosk david@10.42.0.127:

# scp target/aarch64-unknown-linux-gnu/release/onthoudhet-kiosk david@raspberrypi.fritz.box:
