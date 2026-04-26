#!/bin/bash
set -e

echo "Updating pacman and installing base-devel"
pacman -Syu --noconfirm base-devel

echo "Creating build user"
useradd -m builder 2>/dev/null || true
echo "builder ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/builder

echo "Copying PKGBUILD to build directory"
mkdir -p /home/builder/pkgbuild
cp /home/builder/aur/PKGBUILD /home/builder/pkgbuild/PKGBUILD
chown -R builder:builder /home/builder/pkgbuild

echo "Setting up cargo directory for builder user"
mkdir -p /home/builder/.cargo
chown -R builder:builder /home/builder/.cargo

echo "Running makepkg as builder user"
su - builder -c "cd ~/pkgbuild && makepkg -si --noconfirm"

echo "Done!"
