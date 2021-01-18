#!/usr/bin/env nix-shell
#!nix-shell -i bash -p genpass vault wl-clipboard bat libqrencode
# shellcheck shell=bash

set -eu

# Following shm code code lifted from pass:
# Copyright (C) 2012 - 2018 Jason A. Donenfeld <Jason@zx2c4.com>. All Rights Reserved.
# This snippet is licensed under the GPLv2+. Please see pass's COPYING for more information.
if [[ -d /dev/shm && -w /dev/shm && -x /dev/shm ]]; then
    SECURE_TMPDIR="$(mktemp -d "/dev/shm/vpass.XXXXXXXXXX")"
    remove_tmpfile() {
        rm -rf "$SECURE_TMPDIR"
    }
    trap remove_tmpfile EXIT
else
    echo "Your system doesn't have /dev/shm, not continuing."
    exit 1
fi

confirm() {
    read -r -p "$1 [y/N] " response
    if [[ $response == [yY] ]]; then
        return 0
    else
        exit 1
    fi
}

vread() (
    vault read -field=data "password-store/$1"
)
vwrite() (
    vault write "password-store/$1" data=- >&2
)

main() (
    cmd="${1:-help}"
    shift

    case "$cmd" in
    "help")
        echo "Sorry, this isn't going to be helpful, but:"
        bat "$0"
        exit 1
        ;;

    "generate")
        name=$1
        shift
        if vread "$name" >/dev/null; then
            confirm "A password already exists for $name. Overwrite?"
        fi
        genpass "$@" | vwrite "$name"
        exit 0
        ;;

    "edit")
        name=$1
        shift
        vread "$name" >"$SECURE_TMPDIR/secret"

        "${EDITOR:-vi}" "$SECURE_TMPDIR/secret"

        vwrite "$name" <"$SECURE_TMPDIR/secret"
        exit 0
        ;;

    "qr")
        name=$1
        shift
        vread "$name" | head -n1 | qrencode -t utf8
        exit 0
        ;;

    *)
        name=$cmd
        vread "$name"
        exit 0
        ;;
    esac
)

main "$@"
