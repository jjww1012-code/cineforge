#!/usr/bin/env python3
"""Testet CineForge wie Claude Desktop: startet `cineforge mcp` und spricht JSON-RPC über stdio.

Aufruf: tools/mcp_test_client.py BINARY OUTDIR [steps.json]   (TRUNC=2000 für längere Texte)
steps.json: Liste von {"tool": name, "args": {...}}; Bilder werden als step_N.jpg gespeichert.
"""
import base64, json, subprocess, sys, time

binary, outdir = sys.argv[1], sys.argv[2]
steps = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else []

p = subprocess.Popen([binary, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
nid = 0

def rpc(method, params=None, notify=False):
    global nid
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        nid += 1
        msg["id"] = nid
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    if notify:
        return None
    return json.loads(p.stdout.readline())

r = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}})
print("initialize:", r["result"]["serverInfo"], r["result"]["protocolVersion"])
rpc("notifications/initialized", notify=True)
tools = rpc("tools/list")["result"]["tools"]
print("tools:", [t["name"] for t in tools])

for i, step in enumerate(steps):
    t0 = time.time()
    r = rpc("tools/call", {"name": step["tool"], "arguments": step.get("args", {})})
    if "error" in r:
        print(f"[{i}] {step['tool']}: RPC-FEHLER {r['error']}"); continue
    res = r["result"]
    for c in res["content"]:
        if c["type"] == "text":
            txt = c["text"]
            print(f"[{i}] {step['tool']} ({time.time()-t0:.1f}s){' FEHLER' if res['isError'] else ''}: {txt[:int(__import__("os").environ.get("TRUNC","400"))]}")
        elif c["type"] == "image":
            path = f"{outdir}/step_{i}.jpg"
            open(path, "wb").write(base64.b64decode(c["data"]))
            print(f"[{i}] {step['tool']} ({time.time()-t0:.1f}s): Bild → {path}")
p.stdin.close(); p.wait(timeout=10)
