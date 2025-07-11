# cliprust

cliprust is a clipboard history manager written in Rust, inspired by [cliphist](https://github.com/sentriz/cliphist).

## Features

- Write clipboard changes to history files.
- Recall history with pickers such as **dmenu** or **wofi**.
- Supports all MIME types.
- Only uses pipes.
- Supports generating thumbnails.
- Stores every entry in a file, allowing large files to be stored in history.

## Install

### From Source

cliprust is written in [Rust](https://www.rust-lang.org), so the Rust [toolchain](https://rustup.rs) will be needed to compile it.

```sh
cargo install --git https://github.com/aulimaru/cliprust
```

## Usage

For help, run `cliprust -h`.

### Init

First, initialize with `wl-paste --watch cliprust store` to listen for changes on the primary clipboard and store them. Call this once per session, for example, in the Hyprland config:

```
exec-once = wl-paste --watch cliprust store
```

### Select Old Entry

An example usage with `wofi` to select clipboard history and set the current clipboard to it:

```sh
cliprust list | wofi -d | cliprust decode | wl-copy
```

#### Show Thumbnails

cliprust stores thumbnails interactively every time you copy an image. To display them when listing, use `cliprust -g true list`.

An example usage with `wofi` to show thumbnails:

```sh
cliprust -g true list | wofi -d -I | cliprust decode | wl-copy
```

### Delete Old Entry

```sh
cliprust list | wofi -d | cliprust delete
```

### Clear History

```sh
cliprust clear
```

## Config

A default config will be generated when the code is first executed at `$XDG_CONFIG_HOME/cliprust/config.toml` or `$HOME/.config/cliprust/config.toml`.

Use the config file to set the directory where the history files are stored and to define default values, so you don't need to pass arguments each time.

For example, to make cliprust show thumbnails by default, set:

```toml
generate_thumb = true
```
## Useful Tips

### Preventing `wofi` from Sorting Entries

By default, `wofi` may sort entries either alphabetically or by its cache. If you prefer to display entries in the order they are produced (e.g., by `cliprust list`), you can disable caching by setting the cache file to `/dev/null`. This ensures `wofi` does not use its default cache file (located at `$XDG_CACHE_HOME/wofi-dmenu` or `~/.cache/wofi-dmenu` when using dmenu mode).

**Example:**
```sh
cliprust list | wofi -d -k /dev/null | cliprust decode | wl-copy
```

### Preventing Clipboard Overwrite on Quit

If you quit `wofi` without selecting an entry, the result is empty and will overwrite your clipboard. To avoid unintentionally clearing your clipboard, wrap your pipeline with a guard clause:

**Example:**
```sh
cliprust list | wofi -d | { read -r output && cliprust decode <<< "$output" | wl-copy; }
```
This way, the clipboard is only updated if a selection is made.

### Excluding Sensitive Data (e.g., KeePassXC) from Clipboard History

To avoid recording sensitive contents, such as those from password managers like KeePassXC, you can filter clipboard entries based on their MIME types. For instance, KeePassXC marks its clipboard contents with the MIME type `x-kde-passwordManagerHint`. The following script stores clipboard contents only if that hint is absent:

**Example Script:**
```sh
#!/bin/bash

tmpfile=$(mktemp /dev/shm/cliprust.XXXXXX)
chmod 600 "$tmpfile"
trap 'shred -u "$tmpfile" 2>/dev/null' EXIT
cat >"$tmpfile"
if ! wl-paste --list-types | rg -q "x-kde-passwordManagerHint"; then
    cliprust store <"$tmpfile"
fi
```

Save this script (e.g., as `/path/to/cliprust.sh`) and initialize it to watch clipboard changes:

```sh
wl-paste --watch /path/to/cliprust.sh
```

This ensures sensitive clipboard items are not stored in your history.
