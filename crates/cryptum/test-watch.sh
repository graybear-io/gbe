#!/bin/sh
cd /mnt/projects/gbe/gbe-cryptum
echo "starting in $(pwd)"
rm -f .ark-rebuild
while true; do
  echo "checking..."
  if [ -f .ark-rebuild ]; then
    echo "FOUND IT"
    rm -f .ark-rebuild
  fi
  sleep 1
done
