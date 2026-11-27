# Installing and updating

Wordsmith Manuscript is free and open source. Every release is published on the app's
[releases page](https://github.com/jamescjfoundation/wordsmith-manuscript/releases), and nowhere
else.

## The download for your computer

A release has one file for each system, and a list of their checksums. `X.Y.Z` stands for the
version.

| Your computer | File |
| --- | --- |
| Linux, 64-bit Intel or AMD | `wordsmith-manuscript-X.Y.Z-linux-x86_64.tar.gz` |
| Mac with Apple silicon | `wordsmith-manuscript-X.Y.Z-macos-aarch64.dmg` |
| Windows, 64-bit | `wordsmith-manuscript-X.Y.Z-windows-x64-portable.zip` |
| Any | `SHA256SUMS.txt`, the checksums of the three |

Each holds the app (`wordsmith-manuscript`), the command line (`wordsmith-manuscript-cli`, see
[The command line and agents](command-line-and-agents.md)) and the license texts.

The builds are not signed with a publisher's certificate yet. macOS and Windows therefore ask
before they open the app the first time; the steps below say what to choose. To be sure a file
is the one this project published, check it first ([Checking a download](#checking-a-download)).

### Linux

The package needs the C library of Ubuntu 24.04 (glibc 2.39) or a newer one.

1. Unpack the file: `tar -xzf wordsmith-manuscript-X.Y.Z-linux-x86_64.tar.gz`.
2. In the folder it makes, run `./install.sh`. It copies the programs into `~/.local/bin` and the
   menu entry, the icons and the project file types into `~/.local/share`. Nothing needs
   administrator rights. `PREFIX=<folder> ./install.sh` installs under another folder.
3. Start Wordsmith Manuscript from your menu, or run `wordsmith-manuscript`.

`./uninstall.sh` in the same folder removes those files again. Your projects and preferences
are left alone.

### macOS

For a Mac with Apple silicon, on macOS 11 or later.

1. Open the `.dmg` file and drag Wordsmith Manuscript onto the Applications folder beside it.
2. Open the app. The first time, macOS says it cannot check the app and does not open it.
3. Open System Settings › Privacy & Security, find the line about Wordsmith Manuscript and choose
   Open Anyway. On macOS 14 and earlier you can instead hold Control, click the app, choose Open,
   then Open again.

After that it opens like any other app. The command line is inside the app:
`/Applications/Wordsmith Manuscript.app/Contents/MacOS/wordsmith-manuscript-cli`.

### Windows

1. Unpack the `.zip` file anywhere you like: a folder in your Documents, a USB stick. Nothing is
   installed system-wide.
2. Open `wordsmith-manuscript.exe` in the folder. The first time, Windows shows "Windows
   protected your PC": choose More info, then Run anyway.
3. To open projects from File Explorer, run `register-file-types.cmd` in the same folder. It
   registers `.wsm` and `.wsmz` for your account, with the program where it now is; run it again
   if you move the folder. `unregister-file-types.cmd` takes the registration away.

## Checking a download

`SHA256SUMS.txt` lists the SHA-256 checksum of every file of the release. With the file and the
list in one folder:

```sh
sha256sum --check --ignore-missing SHA256SUMS.txt        # Linux
shasum -a 256 --check --ignore-missing SHA256SUMS.txt    # macOS
```

On Windows, `Get-FileHash <file>` in PowerShell prints the checksum to compare with the list.

Every file also has a build provenance attestation: a signed statement, kept by GitHub, that the
file was built from this repository by the release workflow. With the
[GitHub CLI](https://cli.github.com/):

```sh
gh attestation verify wordsmith-manuscript-X.Y.Z-linux-x86_64.tar.gz \
  --repo jamescjfoundation/wordsmith-manuscript \
  --signer-repo jamescjfoundation/open-source-core
```

`--signer-repo` names open-source-core because the jobs that build the files are that
repository's shared release workflow.

## Updates

When the app starts it looks for a newer release, and tells you when there is one. File › About
shows the version you have, with Check for Updates to look at once.

| System | What an update does |
| --- | --- |
| Linux | A copy installed under your home folder downloads the new package, checks it against the release's checksums, replaces itself and starts the new version when you choose Install and Restart. A copy somewhere a package manager or an administrator put it is shown the release page |
| macOS | The app shows the release page; download the new `.dmg` and replace the app |
| Windows | The app shows the release page; download the new `.zip` and unpack it over the old folder, or beside it. Installing in place needs a signed build, which there is not yet |

Your work is saved before the app closes for an update, as it is whenever you close it.

## What the app sends

- **Nothing about you or your writing.** The app has no telemetry: it does not report usage,
  crashes or anything else, and your text, notes, bible and file names never leave your
  computer.
- **The update check is the only request the app makes.** When it starts, and when you choose
  Check for Updates, it asks GitHub (`api.github.com`) for this app's list of releases. GitHub
  sees your computer's network address and a line that names the app and its version, and
  nothing else. An update is downloaded only when you choose to install it, from the release on
  `github.com`.
- **To turn the check off**, clear "Check for updates when the app starts" in File › Preferences
  (it is also on File › About). Nothing is then sent unless you choose Check for Updates
  yourself.
- **The command line never checks.** `wordsmith-manuscript-cli` makes no network request at all.
- **A converter program** runs only when you name one yourself in the export settings
  ([Exporting](export.md)), and only on your computer.

## What the app keeps on your computer

Beside your projects, the app keeps a few files of its own in its settings folder:

| System | Settings folder |
| --- | --- |
| Linux | `~/.config/wordsmith-manuscript` |
| macOS | `~/Library/Application Support/Wordsmith Manuscript` |
| Windows | `%APPDATA%\Wordsmith Manuscript` |

- `preferences.json`: your preferences.
- `history/`: the writing history ([Planning](planning.md)), the words you wrote each day. It is
  kept here and never in the project, so it is not sent along when you share a project.
- `templates/`: your own project templates ([Getting started](getting-started.md)).

Deleting the folder puts the app back as it was when first started. Your projects are not in it.
