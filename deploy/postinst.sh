#!/bin/sh
set -e

DIR="/var/lib/xycrd/"

if [ "$1" = "configure" ]; then
  if ! id -u xymon > /dev/null 2>&1; then
    echo "User xymon not found, unable to chown output directory $DIR"
    exit 0
  fi
  
  chown xymon "${DIR}"
fi
