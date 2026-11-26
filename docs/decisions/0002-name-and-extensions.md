# 0002: The name and the file extensions

Date: 2026-11-24 · Status: accepted

## Context

The app's name, "Wordsmith Manuscript", was confirmed when the plan's open questions were answered
(plan section 13). A project folder and its packed single-file form need extensions that the
operating systems do not already give to a common file type, so that double-clicking opens the
right app and the system does not show a wrong icon or kind.

## Decision

- The name is **Wordsmith Manuscript**; the app id is `org.jcjfoundation.manuscript`.
- A project folder ends in **`.wsm`**; the packed single file ends in **`.wsmz`**.
- The check, made on 2026-11-24:
  - **Linux:** neither extension is in the freedesktop shared MIME database
    (`/usr/share/mime/packages/freedesktop.org.xml`) nor in `/etc/mime.types`. The app installs its
    own types: `application/x-wordsmith-manuscript` (a folder holding `project.toml`) and
    `application/x-wordsmith-manuscript-packed` (a zip).
  - **macOS:** no system uniform type identifier uses either extension. The app declares
    `org.jcjfoundation.manuscript.project` (conforming to `com.apple.package`, so Finder shows
    the folder as one file) and `org.jcjfoundation.manuscript.packed` (conforming to
    `public.zip-archive`).
  - **Windows:** Windows registers neither extension. Public extension lists record `.wsm` only
    for a few unrelated, mostly discontinued third-party formats (installer merge modules,
    interface skins, a modelling tool) that are not installed by default; `.wsmz` has no
    recorded use. On Windows the user opens `project.toml` inside the folder or the `.wsmz` file;
    the folder's extension is never registered there, so no conflict can arise.
- The extensions stay as the plan proposed; the plan does not change.

## Consequences

- Installers (M8.4) register the two types above on each platform.
- If a user has one of the rare unrelated programs that claims `.wsm` on Windows, it has no
  effect: Windows does not open folders by extension.

## Revisit when

A common program or an operating system starts using either extension.
