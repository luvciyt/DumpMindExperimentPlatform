#!/bin/bash

set -euo pipefail

SCRIPT_DIR=$(dirname "$(realpath "$0")")
ROOT_DIR=$(dirname "$SCRIPT_DIR")
IMAGE_DIR="$ROOT_DIR/image"
WORK_DIR="$ROOT_DIR/workspace"
ID=$1
COMMIT_ID=$2
LINUX_WORK_DIR="$WORK_DIR/$ID"
LINUX_BUILD_DIR="$WORK_DIR/$ID/build"
LINUX_INSTALL_DIR="$WORK_DIR/$ID/install"
LINUX_SRC_DIR="$WORK_DIR/$ID/linux-$COMMIT_ID"
LINUX_IMAGE_DIR="$LINUX_WORK_DIR/image"
IMAGE_PATH="$LINUX_IMAGE_DIR/debian.img"
MNT_DIR="$LINUX_IMAGE_DIR/mnt"
LOG_DIR="$LINUX_IMAGE_DIR"
LOG_PATH="$LOG_DIR/$ID.log"

if [[ -z "$COMMIT_ID" ]]; then
  echo "缺少 COMMIT_ID 参数"
  exit 1
fi

if [[ -z "$ID" ]]; then
  echo "缺少 ID 参数"
  exit 1
fi

if [[ ! -f "$IMAGE_PATH" ]]; then
  echo "镜像文件不存在: $IMAGE_PATH"
  exit 1
fi

if [[ ! -d "$MNT_DIR" ]]; then
  mkdir -p "$MNT_DIR"
fi

if mountpoint -q "$MNT_DIR"; then
  sudo umount "$MNT_DIR"
fi

sudo mount -o loop "$IMAGE_PATH" "$MNT_DIR"
trap 'sudo umount "$MNT_DIR" || true' EXIT

VMCORE_PATH="$MNT_DIR/var/crash/vmcore"
if [[ -f "$VMCORE_PATH" ]]; then
  mkdir -p "$LINUX_BUILD_DIR"
  sudo chmod 644 "$VMCORE_PATH"
  echo "改变 vmcore 权限为644"
  sudo mv "$VMCORE_PATH" "$LINUX_BUILD_DIR"
  echo "vmcore 已移动到: $LINUX_BUILD_DIR"
else
  echo "未找到 vmcore 文件: $VMCORE_PATH"
fi

if [[ -f "$LOG_PATH" ]]; then
  mv "$LOG_PATH" "$LINUX_BUILD_DIR"
  echo "日志目录已移动到: $LINUX_BUILD_DIR"
else
  echo "日志目录不存在: $LOG_PATH"
fi
