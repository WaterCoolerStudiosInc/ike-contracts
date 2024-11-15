# Convert wasm contract to standard_input_json
# Example
# python convert.py --manifest xx.toml

import argparse
import os
import json

parser = argparse.ArgumentParser(description='Print all files in the specified directory')

parser.add_argument('--manifest', type=str, default="Cargo.toml", help='manifest path')
parser.add_argument('--exclude', type=str, nargs='+', default=[], help='exclude folder')

args = parser.parse_args()
extension = ["rs", "toml"]
exclude = [".idea", ".git", "target"] + args.exclude
includeFiles = ["Cargo.lock"]

inputJson = {
    "manifest-path": args.manifest,
    "contracts": {}
}

for root, dirs, files in os.walk("."):
    dirs[:] = [d for d in dirs if d not in exclude]
    for file in files:
        if file.endswith(tuple(extension)) or file in includeFiles:
            filePath = os.path.join(root, file)
            with open(filePath, 'r') as f:
                contents = f.read()
                if contents == "":
                    continue
            inputJson["contracts"][filePath.lstrip("./")] = contents

print(json.dumps(inputJson, sort_keys=True, indent=4, separators=(',', ':')))