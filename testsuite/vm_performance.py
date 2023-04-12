#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import re

# Set the tps threshold for block size 1k and 10k
THRESHOLD_1k = 10000
THRESHOLD_10k = 12000

# Run the VM sequential execution with performance optimizations enabled
target_directory = "aptos-move/aptos-transaction-benchmarks/src/"
command = "cargo run --profile performance main false true"
output = subprocess.check_output(command, shell=True, text=True, cwd=target_directory)
print(output)

# Parse the numbers from the output using regex
tps_1k = int(re.findall(r"Avg Sequential TPS = (\d+)", output)[0])
tps_10k = int(re.findall(r"Avg Sequential TPS = (\d+)", output)[1])

print(f"Average Sequential TPS for 1k block: {tps_1k}, Threshold TPS: {THRESHOLD_1k}")
print(f"Average Sequential TPS for 10k block: {tps_10k}, Threshold TPS: {THRESHOLD_10k}")

# Check if any threshold is not met
if tps_1k < THRESHOLD_1k or tps_10k < THRESHOLD_10k:
    print("Sequential TPS below the threshold")
    exit(1)
else:
    print("Sequential TPS above the threshold")
    exit(0)
