# Installing modules

This guide assumes nothing. If you have never installed a module in any Bible
program, start at the top and follow it in order. Every button name below is
written exactly as it appears in GRT Bible.

---

## 1. What a module is

A module is one file that holds one book of material. GRT Bible reads five
kinds:

| Kind | What it is | Where you see it |
|---|---|---|
| Bible | a translation, verse by verse | the translation list in the toolbar |
| Commentary | notes on passages, written by an author | Library, **This passage**, under Commentary |
| Cross-references | for each verse, other verses that speak of the same thing | Library, **This passage**, under Cross-references |
| Dictionary or lexicon | entries by headword, such as Easton's Bible Dictionary or Strong's | Library, **Dictionary** |
| Book | a work divided into parts and sections, such as Josephus | Library, **Books** |

Modules come in three formats:

- **`.grtb`**: GRT Bible's own format. One file, ready to install as it is.
- **SWORD**: the format of CrossWire and of programs such as Xiphos and
  BibleTime. A SWORD module is a `.zip` file with a folder called `mods.d`
  inside it. GRT Bible converts it into a `.grtb` when you install it.
- **USFM**: plain text files used by Bible societies, one per biblical book,
  usually inside a `.zip`. Only translations come in this format.

Nothing is ever downloaded by GRT Bible itself. You download a file with your
browser, or copy it from another computer, and then give it to GRT Bible.

---

## 2. What you already have

The eight translations listed in the README come inside the program. You do
not need to install them, and they cannot be removed.

Each release on GitHub also carries study modules as separate `.grtb` files,
beside the program packages. Today that is:

| File | Kind | Size | Licence |
|---|---|---|---|
| `TSK.grtb` | Cross-references: the Treasury of Scripture Knowledge, about 66,000 groups of references | 8 MB | Public domain |

The release's `SHA256SUMS` file covers these too.

---

## 3. Where to find more

### SWORD modules from CrossWire

CrossWire publishes hundreds of free modules. The list, by kind, is at
`https://www.crosswire.org/sword/modules/`. Every module has a page that tells
you its kind, its size and its licence, for example
`https://www.crosswire.org/sword/modules/ModInfo.jsp?modName=TSK`.

The file itself is always at this address, with the module's name in place of
`NAME`:

```
https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/NAME.zip
```

Some that work well with GRT Bible:

| Name | Kind | Size of the download |
|---|---|---|
| `TSK` | Commentary that GRT Bible offers to read as cross-references | 2.6 MB |
| `Catena` | Commentary: the Catena Aurea of Thomas Aquinas, on the Gospels | 2.3 MB |
| `MHC` | Commentary: Matthew Henry, the whole Bible | 15 MB |
| `Easton` | Dictionary: Easton's Bible Dictionary | 1.4 MB |
| `Nave` | Dictionary: Nave's Topical Bible | 1.3 MB |
| `StrongsGreek` | Lexicon: Strong's Greek dictionary | 0.6 MB |
| `StrongsHebrew` | Lexicon: Strong's Hebrew dictionary | 0.5 MB |
| `Josephus` | Book: the complete works of Flavius Josephus | 1.6 MB |
| `DRC` | Bible: Douay-Rheims, Challoner revision | 1.7 MB |

**Check the licence before you install.** On the module's page, or inside the
`.zip` in the file `mods.d/NAME.conf`, the line `DistributionLicense=` says
what you may do with it. GRT Bible shows the same line in the library after
installing. Only install modules you have the right to use.

Some modules are sold and locked with a key (`CipherKey` in their `.conf`).
GRT Bible does not open locked modules.

### USFM translations from eBible.org

`https://ebible.org/download.php` lists translations in many languages. Choose
the **USFM** download of a translation: it is a `.zip` whose name ends in
`_usfm.zip`.

---

## 4. Installing on Linux

### 4.1 Download the file

1. Open your web browser (Firefox, for example).
2. Go to the address of the module, for example
   `https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Easton.zip`,
   or to the release page for a `.grtb` file.
3. The browser saves the file in your **Downloads** folder. On an Italian
   desktop that folder is called **Scaricati**; on other languages it has the
   name of that language. It is the folder your browser opens when you click
   its downloads list.

From a terminal, the same download is one command:

```bash
curl -L -o ~/Downloads/Easton.zip https://www.crosswire.org/ftpmirror/pub/sword/packages/rawzip/Easton.zip
```

If you installed GRT Bible from the `.deb` package, keep the file in your
**Downloads** or **Documents** folder. The `.deb` installs a security profile
that lets GRT Bible read files there and in what you choose through the file
dialog, and nowhere else. The AppImage and a local build have no such limit.

### 4.2 Install it

1. Open GRT Bible.
2. In the toolbar, click **Library**. A panel opens on the right.
3. At the top of the panel, click the tab **Installed**.
4. Then:
   - for a `.grtb` file, click **Install a .grtb file…**;
   - for a SWORD `.zip` or a USFM `.zip`, click
     **Convert a SWORD or USFM module…**, then **Choose a file…**.
     A SWORD module that you have unpacked into a folder can be chosen with
     **Choose a folder…**: pick the folder that contains `mods.d`.
5. A file dialog opens. Go to **Downloads** (or **Scaricati**), click the file,
   then click **Open**.
6. For a `.grtb` file, GRT Bible checks it and installs it. A message at the top
   says `NAME installed:` followed by what it holds. You are done.
7. For a SWORD or USFM file, a form opens with what GRT Bible found in the file:
   - **Read it as** appears only for a commentary. Leave the suggested choice.
     For the Treasury of Scripture Knowledge it is **Cross-references**, which
     is what you want.
   - **Abbreviation**: a short name, 2 to 10 letters or digits, such as `EAS`.
     It must be different from every module already installed.
   - **Title**: the full name. It is filled in from the module.
   - **Language code**: `en` for English, `it` for Italian, `la` for Latin.
     It is filled in when the module says it.
   - **Verse numbering** appears for Bibles, commentaries and cross-references.
     Leave the suggested choice unless you know it is wrong: it decides which
     verse each note and reference lands on.
8. Click **Import**. The text `Converting… this can take a minute.` appears.
   A small dictionary takes a second; Matthew Henry's commentary takes a few.
9. When it is done the form closes and the message `NAME installed.` appears.

### 4.3 Check that it arrived

1. Still in **Library**, tab **Installed**, the new module has a card under its
   kind: **Commentaries**, **Cross-references**, **Dictionaries and lexicons**,
   **Books** or **Bibles**.
2. The card says its licence, its source, and
   `Installed from FILE on DATE.` with the name of the file you chose.
3. Then use it:
   - a commentary or cross-references: select a verse in the text (click it),
     then click **References** in the bar that appears at the bottom. The tab
     **This passage** shows them;
   - a dictionary: tab **Dictionary**, type the first letters of a word;
   - a book: tab **Books**, then click a part to open it.

Installed modules live in `~/.local/share/org.grt.bible/modules/`. You do not
need to touch that folder; it is where to look if you want a backup.

---

## 5. Installing on Windows

### 5.1 Download the file

1. Open your web browser (Edge, for example).
2. Go to the address of the module or to the release page, as in section 3.
3. The file is saved in your **Downloads** folder, the one File Explorer shows
   under **This PC**. Edge shows it in the downloads list at the top right.

Windows may ask whether to keep a file it does not know. Choose **Keep**. A
`.grtb` or a `.zip` of a module is data, not a program.

### 5.2 Install it

1. Open GRT Bible from the Start menu.
2. Click **Library** in the toolbar, then the tab **Installed**.
3. Click **Install a .grtb file…** for a `.grtb`, or
   **Convert a SWORD or USFM module…** and then **Choose a file…** for a
   `.zip`.
4. In the window that opens, click **Downloads** on the left, select the file
   and click **Open**.
5. For a `.zip`, fill in the form as described in step 7 of section 4.2, then
   click **Import**.

### 5.3 Check that it arrived

As in section 4.3. Installed modules live in
`%APPDATA%\org.grt.bible\modules\`: type that into the address bar of File
Explorer to open it.

---

## 6. Installing on Android

### 6.1 Put the file on the phone

Either:

- **Download it on the phone.** Open Chrome, or any browser, and go to the
  address of the module or to the release page. The file goes into the
  phone's **Download** folder. Chrome shows it under **Downloads** in its menu.

or:

- **Copy it from a computer.**
  1. Connect the phone with a USB cable.
  2. On the phone, pull down the notifications, tap the USB notification and
     choose **File transfer**.
  3. On the computer the phone appears as a drive. Open it, open
     **Internal storage** (or **Internal shared storage**), then the folder
     **Download**, and copy the file there.

### 6.2 Install it

1. Open GRT Bible.
2. In the bar at the bottom, tap **Library**.
3. Tap the tab **Installed**.
4. Tap **Install a .grtb file…** for a `.grtb`, or
   **Convert a SWORD or USFM module…** and then **Choose a file…** for a
   `.zip`.
5. Android's file picker opens. It shows **every file**, not only modules: that
   is expected, because Android does not know the `.grtb` type and cannot
   filter by it. Tap the menu (three lines) at the top left, tap **Downloads**,
   then tap your file.
6. For a `.zip`, the form described in step 7 of section 4.2 opens. Fill it in
   and tap **Import**. Converting on a phone is slower than on a computer:
   Matthew Henry's commentary can take a minute. Keep GRT Bible open meanwhile.

### 6.3 Check that it arrived

As in section 4.3: the card appears in **Installed**, and the material appears
in **This passage**, **Dictionary** or **Books**.

On Android the installed modules live inside the application and are not
visible from the file manager. Uninstalling GRT Bible deletes them; the files
you downloaded stay in **Download**.

---

## 7. When a file is refused

GRT Bible says why. What each message means:

| Message | What it means | What to do |
|---|---|---|
| `This file is not a GRT Bible module (.grtb).` | The file you chose with **Install a .grtb file…** is something else, often a `.zip`. | Use **Convert a SWORD or USFM module…** for a `.zip`. |
| `A module called NAME is already installed. Remove it first to install this one.` | Abbreviations must be unique. | Remove the old one (section 8), or for a conversion choose another **Abbreviation**. |
| `no mods.d/*.conf file: this does not look like a SWORD module` | The `.zip` has no `mods.d` folder. | Check that you downloaded the module and not a web page; a USFM `.zip` is fine. |
| `this module is locked with a cipher key and cannot be imported` | A sold module. | GRT Bible does not open these. |
| `'NAME' uses the driver X, which GRT Bible does not read.` | A kind of SWORD module GRT Bible does not read yet, such as images or personal commentaries. | None for now. |
| `the module holds nothing GRT Bible can read` | The module is empty, or every entry is outside the 73 books of the Catholic canon. | None. |
| `NAME needs a newer GRT Bible (schema N)` | The `.grtb` was made by a later version. | Update GRT Bible. |
| `module kind 'X' is not one this version reads` | The `.grtb` holds a kind of material added in a later version. | Update GRT Bible. |
| `the module file is damaged: ...` | The file was cut short or changed while downloading. | Download it again and compare it with `SHA256SUMS`. |
| `the .conf points outside the module` | The module's `.conf` names a folder outside itself. | Do not use that file. |

On Linux, if you check a download from the release page:

```bash
cd ~/Downloads
sha256sum -c SHA256SUMS --ignore-missing
```

Each line must end in `OK`.

---

## 8. Removing a module

1. **Library**, tab **Installed**.
2. On the module's card, click **Remove**, then confirm.

Your notes, highlights and bookmarks are not touched: they belong to verses,
not to modules. Translations that come with the program have no **Remove**
button.

---

## 9. Making a `.grtb` yourself

For people who build GRT Bible from source. The converter writes the same file
the program writes when it imports:

```bash
cargo build --release -p grtb-convert
target/release/grtb-convert sword Easton.zip \
  --id eng-easton --abbreviation EAS --title "Easton's Bible Dictionary" \
  --language en --license "Public domain" -o EAS.grtb
target/release/grtb-convert info EAS.grtb
```

`--as crossrefs` reads a commentary as cross-references. The same input always
gives the same bytes, so a module can be checked by its checksum.
`scripts/convert-packs.sh` builds the packs published with each release.
