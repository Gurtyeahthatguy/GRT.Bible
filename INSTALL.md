# Installing GRT Bible

The program is at version **0.1.0**, and the same version runs on Linux, on
Windows and on Android.

The eight translations are inside every package. Nothing is downloaded on first
run, and nothing has to be set up afterwards.

---

## The short way: download one

Every release carries ready-made files. Nothing else has to be installed to use
them.

**Linux.** Take the `.AppImage`. It is one file, it needs no installation and no
administrator rights, and it runs from anywhere:

```bash
chmod +x grt-bible-0.1.0-linux-x86_64.AppImage
./grt-bible-0.1.0-linux-x86_64.AppImage
```

The `.deb` and `.rpm` beside it install through the package manager instead,
which puts the program in the application menu:

```bash
sudo apt install ./grt-bible-0.1.0-linux-x86_64.deb     # Debian, Ubuntu
sudo dnf install ./grt-bible-0.1.0-linux-x86_64.rpm     # Fedora
```

**Windows.** Take `grt-bible-0.1.0-windows-x86_64-setup.exe`. It is not
code-signed, so Windows will warn about an unknown publisher.

**Android.** Take the `.apk` and copy it to the phone, or install it over a
cable:

```bash
adb install grt-bible-0.1.0-android-arm64.apk
```

Opening it on the phone asks once for permission to install an application that
does not come from a store. It needs Android 7 or newer and a 64-bit processor,
which is every phone sold since 2017. The first start takes a few seconds while
the translations are unpacked out of the package.

---

## Building from source

Requires Node 22 and a Rust toolchain.

**Linux** also needs the WebKitGTK development libraries:

```bash
sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev build-essential curl file libssl-dev librsvg2-dev
```

**Windows** needs the Microsoft C++ build tools, which the Rust installer offers
to fetch, and WebView2, which Windows 10 and 11 already have.

Then, on either:

```bash
npm install
npm run build            # packages for this system
npm run build:binary     # just the executable
```

The result is `target/release/grt-bible`, or `grt-bible.exe` on Windows, and the
packages under `target/release/bundle/`.

Build through those scripts rather than `cargo build --release` directly. A bare
cargo build produces a working program whose binary contains the absolute path
of the directory it was compiled in, which `scripts/check-build.sh` will tell you
about.

On Linux, to put a locally built copy in the application menu without root:

```bash
./scripts/install-local.sh
```

### Android

Three more things are needed: a JDK 17, the Android SDK with its command line
tools, and the NDK. `npm run build:android` looks for them in `~/Android/jdk-17`
and `~/Android/Sdk`, or wherever `JAVA_HOME`, `ANDROID_HOME` and `NDK_HOME`
point.

A fresh checkout also needs the Android project filled in once, which writes the
generated files the repository leaves out and touches nothing that is in it:

```bash
npx tauri android init
```

Then, for the build itself:

```bash
rustup target add aarch64-linux-android
npm run build:android
```

The APK lands in
`src-tauri/gen/android/app/build/outputs/apk/universal/release/`. Add
`--target x86_64` for one an emulator can run.

A release APK has to be signed, and a phone only accepts an update signed with
the same key as the version it already has. Create a key once, keep it
somewhere safe, and name it in `src-tauri/gen/android/keystore.properties`,
which the repository ignores:

```bash
keytool -genkeypair -keystore ~/.android/grt-bible-release.jks -storetype PKCS12 \
  -alias grt-bible -keyalg RSA -keysize 4096 -validity 36500 -dname "CN=GRT Project"
```

```
storeFile=/home/you/.android/grt-bible-release.jks
keyAlias=grt-bible
password=whatever you typed
```

Without that file the build still runs and leaves the APK unsigned, which no
phone will install.

### Rebuilding the translations

The `.grtb` files in `modules/` are generated and committed, so a normal build
does not need their sources. To rebuild them, put the texts listed in the README
under `sources/` and run:

```bash
./scripts/convert-modules.sh
```

The converter is deterministic: the same source gives the same file, byte for
byte. To see what a module holds and what it is missing:

```bash
cargo run --release -p grtb-convert -- info modules/MAR.grtb
```

---

## Where your data is

Reading position, history, notes, highlights, bookmarks and saved videos are in
one file, `user.grt`. Imported translations and an imported video catalog sit
beside it.

| | Linux | Windows | Android |
|---|---|---|---|
| Your file and imported modules | `~/.local/share/org.grt.bible/` | `%APPDATA%\org.grt.bible\` | inside the application, not reachable from the file manager |
| Settings | `~/.config/org.grt.bible/settings.json` | `%APPDATA%\org.grt.bible\settings.json` | inside the application |

On Android nothing is copied to a Google account: backup is switched off. Export
from Settings to take the file off the phone.

---

## Removing it

**Linux**, if it was installed with the script:

```bash
./scripts/install-local.sh --remove
```

If it came from the `.deb` or the `.rpm`, remove it through the package manager.
The AppImage is one file: delete it.

**Windows.** Through Settings, Apps.

**Android.** Hold the icon, then Uninstall. That deletes your `.grt` file with
it, so export it first if it matters.

What is left behind on purpose on Linux and Windows, because it is yours, are
the folders listed above. Delete them by hand to leave nothing.
