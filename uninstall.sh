#!/bin/sh
# Remove func and its completions. Your scripts are kept unless you pass --purge.
#
#   ./uninstall.sh                 remove func from ~/.local/bin
#   ./uninstall.sh --prefix /usr/local
#   ./uninstall.sh --purge         also delete saved scripts and the build cache

set -eu

bin_dir="${HOME:-}/.local/bin"
purge=no

while [ $# -gt 0 ]; do
    case $1 in
        --prefix) [ $# -ge 2 ] || { echo "--prefix needs a directory" >&2; exit 2; }; bin_dir="$2/bin"; shift 2 ;;
        --purge) purge=yes; shift ;;
        -h | --help) sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "I do not know the option \`$1\`. Run ./uninstall.sh --help." >&2; exit 2 ;;
    esac
done

data="${XDG_DATA_HOME:-${HOME:-}/.local/share}"
config="${XDG_CONFIG_HOME:-${HOME:-}/.config}"
cache="${XDG_CACHE_HOME:-${HOME:-}/.cache}"

remove() {
    if [ -e "$1" ]; then
        rm -rf "$1"
        echo "  removed  $1"
    fi
}

remove "$bin_dir/func"
remove "$data/bash-completion/completions/func"
remove "$data/zsh/site-functions/_func"
remove "$config/fish/completions/func.fish"

if [ "$purge" = yes ]; then
    remove "$data/func"
    remove "$cache/func"
else
    echo "Kept your scripts in $data/func. Use --purge to delete them too."
fi
