# Linux desktop

The Linux desktop release targets **Ubuntu 24.04 on x86_64 and ARM64**, with
glibc 2.39 or newer. Packages for both architectures are built natively.
ARM64 has been exercised in a VM; x86_64, hardware GPUs and the full desktop
acceptance list still need validation. Other distributions, musl and Alpine
are not supported by this desktop package. The separate CLI/server downloads
keep their glibc 2.35 requirement.

Wayland and native X11 are available. Rendering needs a working Vulkan driver
with Vulkan 1.1 and the extensions required by GPUI's Blade renderer. A software
renderer can run the app, but does not establish hardware performance. GNOME
Wayland currently uses the app's fallback window buttons and cannot resize
from its edges. Quick Terminal works on native X11; it is unavailable on
Wayland and XWayland. See the [platform limits](../tech/constraints.md#experimental-linux-desktop)
for clipboard, extension-page input and voice requirements.

## Install

Install the system libraries on Ubuntu 24.04:

```sh
sudo apt-get update
sudo apt-get install python3 ca-certificates libfreetype6 libfontconfig1 libxcb1 \
  libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libvulkan1 mesa-vulkan-drivers \
  libwebkit2gtk-4.1-0 xdg-utils xdg-desktop-portal bubblewrap xdg-dbus-proxy
```

Use your desktop's portal backend too: `xdg-desktop-portal-gnome`,
`xdg-desktop-portal-kde` or `xdg-desktop-portal-gtk`. Use your GPU vendor's
Vulkan driver where appropriate. WebKitGTK 4.1, version 2.40 or newer, is
required for extensions; the integrated browser needs version 2.42 or newer. Voice is optional and needs the additional
[local transcription tools and models](../tech/constraints.md#experimental-linux-desktop).

Choose a release containing `muxy-desktop-<version>-linux-<arch>.tar.gz` from
the [release downloads](https://github.com/muxy-app/muxy/releases). Set
`VERSION` to its version without `v`; use `X.Y.Z-beta.N` for the beta.

```sh
VERSION=2.1.0
curl -fL --proto '=https' --proto-redir '=https' \
  "https://github.com/muxy-app/muxy/releases/download/v$VERSION/install-desktop-linux.py" \
  -o install-desktop-linux.py
python3 install-desktop-linux.py install --version "$VERSION"
```

Run the installer as your normal user, without `sudo`. It selects the native
architecture, downloads the archive over HTTPS, verifies its SHA-256 against
that release's metadata, and checks that the app, CLI and server report the
same build. It does not open the app or start a server.

Open **Muxy** or **Muxy Beta** from your desktop's Applications menu. No terminal
environment or development tools are needed. The installations are separate:

| Channel | Installation | Shell commands |
| --- | --- | --- |
| Stable | `~/.local/share/muxy-desktop` | `~/.local/bin/muxy`, `~/.local/bin/muxy-server` |
| Beta | `~/.local/share/muxy-desktop-beta` | `~/.local/bin/muxy-beta`, `~/.local/bin/muxy-server-beta` |

`XDG_DATA_HOME`, when set to an absolute path, replaces `~/.local/share`.
Settings and session data stay in Muxy's existing profile, outside these
installation directories. An existing command belonging to another installation
is kept, with a message showing the bundled command's full path.

Add `~/.local/bin` to your shell's `PATH`. Use `muxy <folder>` or
`muxy-beta <folder>` to open a project. **Install Command Line Tool…** in the
command palette can also link the running app's CLI as `muxy`, provided that
name is free or already managed by the app.

## Updates and recovery

The app checks automatically, and **Check for Updates** checks immediately.
Stable installs receive stable releases; beta installs receive betas. The
updater verifies the archive checksum, file hashes, architecture and matching
helper versions before replacing anything. It uses the same server protocol
and session rules as macOS: compatible running sessions continue; an
incompatible update waits until sessions end or asks before ending them.
Canceling that confirmation leaves the installation and sessions untouched.

Each release has its own directory. Switching the `current` link replaces the
app and both helpers together. Restart setup failures and immediate startup
failures restore the previous link. One previous release is retained for
recovery, along with older versions still in use by apps, CLIs or servers.
Unused older versions are removed when the app connects to its server.
The app's profile selection is inherited when it restarts.

Updates do not write to `/usr`, replace package-manager files or change
standalone CLI installations. Development builds and manually unpacked
archives do not enable automatic updates. Use the installer for desktop
integration and updates. If an installation is damaged, quit the app and
stop its server, uninstall as below, then install again; profile data stays.

## WebKit sandbox on Ubuntu

Ubuntu restricts unprivileged user namespaces. The app probes whether WebKit's
sandbox can start and uses the existing unsandboxed fallback when it cannot.
The release includes `muxy-desktop.apparmor` to permit the sandbox for the
default installation paths, following Ubuntu's
[application-profile guidance](https://documentation.ubuntu.com/release-notes/24.04/).
An administrator can install it:

```sh
sudo install -m 644 muxy-desktop.apparmor /etc/apparmor.d/muxy-desktop
sudo apparmor_parser -r /etc/apparmor.d/muxy-desktop
```

Download the profile from the same release, adapt its attachment path if using
a custom `XDG_DATA_HOME`, and fully quit and reopen the app after loading it.
This profile grants user-namespace creation; WebKit provides the page sandbox.
It does not confine the whole terminal app. The user-local installer does not
change system security settings.

## Uninstall

Quit the app and stop its local server first. Stopping the server ends its
running terminals. For stable:

```sh
python3 "${XDG_DATA_HOME:-$HOME/.local/share}/muxy-desktop/current/install.py" uninstall
```

For beta, replace `muxy-desktop` with `muxy-desktop-beta`. Removal refuses to
proceed while an installed app, CLI or server is using any version. It removes
the installation, its unchanged desktop entry and only CLI links pointing into
that installation. Settings, projects, sessions, extensions and unrelated CLI
files are preserved. An administrator can separately remove the optional
AppArmor profile when neither channel is installed.

