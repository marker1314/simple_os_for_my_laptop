#!/usr/bin/env bash
set -euo pipefail

echo "[Preflight] Simple OS pre-use sanity checks"

echo "[1/5] Input device check (60s)"
echo " - Move mouse and type keys; ensure no freezes or multi-second lag."
sleep 5
echo "   ...manual observation window (simulate)"
sleep 55 || true

echo "[2/5] Large file I/O (read/write)"
echo " - Create 256MB test file and read it back (if tools available)."
echo "   (On Simple OS shell, use: dd if=/dev/zero of=/tmp/test.bin bs=1M count=256)"
echo "   (Then: dd if=/tmp/test.bin of=/dev/null bs=1M)"

echo "[3/5] Audio 5-min playback"
echo " - Start PCM test tone and observe for underruns/interrupts."
echo "   (In shell: audio_test start; wait 300s; audio_test stop)"

echo "[4/5] Suspend/Resume x10"
echo " - Run: for i in {1..10}; do suspend_s3; sleep 5; resume; done"

echo "[5/5] Network basic checks"
echo " - If Wi‑Fi up: ping gateway, fetch small HTTP, verify reconnect after AP reset"

echo "[Done] Review logs: serial console and /var/log/kernel.log (if present)"


