#!/usr/bin/env python3
"""Compare every registered method's identity/signature and inspect branch targets of its live invoker."""
import argparse
from collections import Counter
import json
from pathlib import Path

from census import HERE, census

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("stock", type=Path)
parser.add_argument("candidate", type=Path)
args = parser.parse_args()
arms = {}
for name, binary in vars(args).items():
    methods, adapter = census(binary, Path(str(binary) + ".map"), HERE / "layout.txt", method_details=True)
    arms[name] = methods, adapter
stock, _ = arms["stock"]
candidate, shared = arms["candidate"]
assert stock.keys() == candidate.keys()
mismatches = [key for key in stock if stock[key]["signature"] != candidate[key]["signature"]]
assert not mismatches, mismatches
assert all(not r["uses_adapter"] for r in stock.values())
assert all(r["routed"] for r in candidate.values() if r["uses_adapter"])
eligible = [k for k, r in candidate.items() if r["uses_adapter"]]
other = [k for k, r in candidate.items() if r["routed"] and not r["uses_adapter"]]
def cohort(methods, keys):
    entries = {methods[key]["entry_address"]: methods[key]["entry_bytes"] for key in keys}
    return {"method_rows":len(keys), "invokers":len(entries), "invoker_text_bytes": sum(entries.values())}
print(json.dumps({
    "method_rows_compared":len(stock), "identity_signature_differences":len(mismatches),
    "specialized":{arm:cohort(methods,eligible) for arm,(methods,_) in arms.items()},
    "matching_signature_without_adapter":{arm:cohort(methods,other) for arm,(methods,_) in arms.items()},
    "unrelated_rows_using_adapter":0, "shared_adapter":shared,
    "other_routed_method_names":dict(sorted(Counter(key[2] for key in other).items()))
}, indent=2))
