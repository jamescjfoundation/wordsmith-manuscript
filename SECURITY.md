# Security

## Reporting a problem

Report security problems privately, not in a public issue, pull request or discussion. Use
GitHub's private reporting: the **Security** tab of this repository, then **Report a
vulnerability**. Only the maintainers see the report.

Please include what you ran, what happened and what you expected. If you attach a project or other
file to show the problem, remove anything personal from it first. A small made-up project is best;
never attach your own manuscript.

We aim to reply within seven days. We will tell you when a fix is ready and credit you in the
advisory and the changelog, unless you ask us not to.

## What counts

Anything that lets a file, an agent or a network peer do more than the user asked for. For example:

- A project folder, packed project (`.wsmz`), DOCX, ODT, RTF, Markdown or text file that crashes
  the app, hangs it, or makes it write somewhere unexpected when opened or imported.
- A packed project that writes outside the folder chosen to unpack it.
- An agent tool reading or writing outside the project it was given.
- The update check installing anything that is not a verified release file.
- Recovery, auto-save or backup files readable by other users.

Only the latest release is supported. Fixes are not backported.

## How we handle a report

1. **Keep it private until the fix ships.** Do not describe the problem in a public issue, pull
   request, commit message, branch name, code comment or file in the repository. Working notes stay
   in `.private/` (ignored by Git).
2. **Fix it in the open, quietly.** The fix lands as a normal change whose message says what the
   code now does, not how to exploit the old code. Add a test that feeds the bad input.
3. **Release, then publish.** After the release, publish the advisory and a changelog line.
