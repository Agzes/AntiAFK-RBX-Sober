# AntiAFK-RBX-Sober GNOME Extension

A lightweight GNOME Shell extension that enables safe window focusing, cursor restoration, and window screenshots for AntiAFK-RBX-Sober on Wayland without using unsafe APIs.

---

## Installation

### Method 1: In the App (Recommended)

1. Launch **AntiAFK-RBX-Sober**.
2. Open **Diagnostics**.
3. Click **Install & Enable** on the **GNOME Extension** row.

### Method 2: Via CLI

```bash
AntiAFK-RBX-Sober --install-gnome-helper
```

### Method 3: Manual

```bash
UUID="antiafk-rbx-sober@agzes.github.io"
DEST="$HOME/.local/share/gnome-shell/extensions/$UUID"
mkdir -p "$DEST"
cp metadata.json extension.js "$DEST/"
gnome-extensions enable "$UUID"
```

---

## Important Note

On GNOME (Wayland), **log out and log back in** after the first installation so GNOME Shell can load the extension.
