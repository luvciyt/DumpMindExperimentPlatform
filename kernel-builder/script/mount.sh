#!/bin/bash

log() {
    local LEVEL=$1
    shift
    echo "[$LEVEL] $(date +'%Y-%m-%d %H:%M:%S') $*"
}

error_exit() {
    log "ERROR" "$1"
    exit 1
}

create_dir() {
    local DIR=$1
    if [ ! -d "$DIR" ]; then
        mkdir -p "$DIR" || error_exit "Failed to create directory: $DIR"
        log "INFO" "Directory created: $DIR"
    fi
}

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

if [ -z "$COMMIT_ID" ]; then
    error_exit "Commit ID is required as an argument."
fi

log "INFO" "Starting process with COMMIT_ID: $COMMIT_ID"

cd "$WORK_DIR" || error_exit "Failed to change directory to $WORK_DIR"
cd "$ID" || error_exit "Failed to change directory to $WORK_DIR/$ID"

create_dir "image"

cd "image" || error_exit "Failed to change directory to $WORK_DIR/$ID/image"

log "INFO" "Copying debian.img..."
rsync -av "$IMAGE_DIR/debian.img" ./ || error_exit "Failed to copy debian.img"

create_dir "mnt"

log "INFO" "Mounting debian.img..."
sudo mount -o loop debian.img mnt || error_exit "Failed to mount debian.img"

log "INFO" "Copying Linux headers..."
cd mnt/usr/include || error_exit "Failed to enter mnt/usr/include"
sudo rm -rf ./asm || error_exit "Failed to remove asm directory"
sudo rm -rf ./linux || error_exit "Failed to remove linux directory"
sudo cp -R "$LINUX_INSTALL_DIR/include/asm" ./ || error_exit "Failed to copy asm headers"
sudo cp -R "$LINUX_INSTALL_DIR/include/linux" ./ || error_exit "Failed to copy linux headers"

log "INFO" "Copying bug.c to root..."
cd ../../ || error_exit "Failed to return to mnt directory"
sudo cp -R "$LINUX_WORK_DIR/bug.c" ./root || error_exit "Failed to copy bug.c"

log "INFO" "Unmounting debian.img..."
cd ..
sudo umount mnt || error_exit "Failed to unmount debian.img"

cp "$LINUX_BUILD_DIR/arch/x86_64/boot/bzImage" ./

log "INFO" "Image has been successfully copied."