
About Meld
==========

Meld is a visual diff and merge tool targeted at developers. Meld helps you
compare files, directories, and version controlled projects. It provides
two- and three-way comparison of both files and directories, and supports
many version control systems including Git, Mercurial, Bazaar, CVS, Darcs
and Subversion.

Meld helps you review code changes, understand patches, and makes enormous
merge conflicts slightly less painful. Git Compare compares any two
revisions of a Git repository, with a Why column saying which commit made
each change, why, and why it is the right change (AI Lens).

This is a rewrite of Meld in Rust with an [iced](https://iced.rs) user
interface. The original Python and GTK version lives on upstream; its last
state in this repository is commit 29d59e07.

A pane with no file behind it highlights what its text looks like: Batch,
C, C++, C#, CSS, Common Lisp, Erlang, Go, Groovy, Haskell, HTML, Java,
JavaScript, JSON, Lua, Markdown, Objective-C, OCaml, Perl, PHP, Python, R,
Ruby, Rust, Scala, Shell, SQL, TypeScript, XML or YAML. Languages the
detector recognises but this build cannot highlight (Dockerfile, TOML and
others) keep the pane as it is. A confident detection switches the
highlighting; inconclusive text keeps the language the pane already shows,
so an unfinished document does not flicker, and for ordinary editing the
highlighting stops only when the pane becomes empty (whitespace-only counts
as empty). Choosing **Auto-detect** re-checks the current text from scratch:
without a conclusive shape it falls back to plain text.
Picking a language in the Highlighting Mode menu — Plain Text included —
turns detection off for that pane until **Auto-detect** is chosen there
again; panes opened from a file keep the language of the file.

Meld is licensed under the GPL v2 or later.


Requirements
------------

* Rust 1.88 or later (the workspace is developed with 1.98.1)
* A C compiler (for the bundled Oniguruma regular expression library)
* A GPU driver with Vulkan, Metal, DirectX 12 or OpenGL support
* At run time, optionally: `git` and other version control tools, `gh` and
  `glab` for pull requests, and Claude Code (`claude`) for AI Lens
  explanations


Building and running
--------------------

```sh
$ cargo run --release -p meld -- [arguments]
```

for example:

```sh
$ cargo run --release -p meld -- file1 file2           # compare files
$ cargo run --release -p meld -- dir1 dir2 dir3        # compare folders
$ cargo run --release -p meld -- .                     # version control view
$ cargo run --release -p meld -- --git --base main     # Git Compare
$ cargo run --release -p meld -- --auto-merge local base remote -o merged
```

`meld --help` lists all options. They match the Python version's, so Meld
still works as a `git mergetool` or `git difftool`. On macOS, shortcuts use
Command (`cmd`) instead of Control (`ctrl`); in folder comparisons, `cmd+Delete`
deletes the selected files while plain Delete (`Backspace`) navigates to the
parent row.

To install a desktop entry and icon for your user, build a release binary
and run `crates/meld/data/install-desktop.sh`.

Settings are stored in `$XDG_CONFIG_HOME/meld/settings.toml` and recent
comparisons in `$XDG_DATA_HOME/meld/recent.toml`.


macOS application bundle
------------------------

`scripts/macos-install.sh` builds Meld and installs it as `Meld_rs.app` in
`/Applications`, with `meld-rs` exposed on your PATH through a symlink in
`~/bin` (it replaces a manually copied `~/bin/meld-rs`):

```sh
$ scripts/macos-install.sh
```

The app icon comes from `data/meld_rs.icns`, the display name is "Meld RS",
and the bundle is registered for files and folders, so Finder's
"Open With Meld_rs" opens comparisons (the CLI accepts up to three paths).
Opening documents while the app is already running is not supported yet.

The `Contents/MacOS/Meld_rs` launcher resolves PATH from your `~/.bashrc`
before starting Meld, so run-time tools (`git`, `gh`, `glab`, `claude`) are
found when the app is launched from Finder, which otherwise inherits only
launchd's minimal PATH. Terminal launches through `meld-rs` behave the same.

`scripts/make-app.sh --binary target/release/meld` assembles the bundle
without installing it; both scripts document their options with `--help`.
The bundle is unsigned and meant for local use: on other Macs Gatekeeper
blocks it unless it is signed and notarized. `scripts/test-macos-bundle.sh`
checks the packaging scripts in an isolated temp directory.


Customizing keyboard shortcuts
------------------------------

Open **Main menu → Keyboard Shortcuts**. Search by action or key, then choose
**Edit** on an action. **Add shortcut** or **Change** starts recording; press a
chord, then choose **Save** to apply it. **Escape** cancels recording without
closing the window. **Remove** takes effect immediately; removing the last
chord leaves the action **Unbound**. **Restore …** brings back one default
without replacing other bindings; **Reset action** restores all its defaults.
Changes use the existing `meld/settings.toml` under the platform config
folder (on macOS, `~/Library/Application Support`).

**Quick macOS check:** Quit the running Meld RS, rebuild from this checkout
with `scripts/macos-install.sh`, then open the rebuilt **Meld RS**. Use an
empty comparison tab and start with default bindings for **New comparison**
and **Open menu**. If macOS handles the suggested function keys itself, use
its standard F-key mode (or Fn) or pick unused alternatives.

1. Search for **New comparison**. Choose **Edit → Add shortcut**, record
   **F6**, then **Save**. Close the window and press F6; a new tab should open.
2. Reopen the editor and choose **Change** next to F6. Record **F10** and
   choose **Save**. With defaults, the editor should name **Open menu** as
   the conflict and leave F6 active. Choose **Record again**, record **F7**
   and save. F7 should now open a tab instead of F6.
3. Quit and reopen Meld RS. Confirm F7 still opens a tab, then choose
   **Reset action**. The platform default should work again and F7 should no
   longer open a tab.


Developing
----------

The workspace is split into crates:

| Crate          | Contents                                                         |
|----------------|------------------------------------------------------------------|
| `meld-diff`    | diff and merge engine: matchers, chunks, merging, patches, undo  |
| `meld-files`   | text loading and encodings, languages, filters, folder comparison|
| `meld-config`  | settings, recent comparisons, style schemes, diff palettes       |
| `meld-vc`      | Git, Mercurial, Subversion, Bazaar, CVS and Darcs backends       |
| `meld-ailens`  | Git Compare core: attribution, explanations, Claude Code client  |
| `meld-widgets` | iced widgets: text panes, link and chunk maps, highlighting      |
| `meld`         | the application                                                  |

```sh
$ cargo test --workspace
```

`crates/PORTING.md` describes the conventions. The version control
integration tests skip tools that aren't installed.


Contributing
------------

Upstream Meld uses GNOME's GitLab to track bugs, and user questions and
development discussions happen on the GNOME Discourse.

List of issues: https://gitlab.gnome.org/GNOME/meld/issues

Support forum:  https://discourse.gnome.org/tag/meld


Links
-----

Home page:      https://meld.app/

Documentation:  https://meld.app/help/


Credits
-------

Meld is licensed under the GPL v2 or later.

Detecting the language of unnamed panes uses [`betlang`](https://crates.io/crates/betlang)
(MIT), which embeds a distilled model trained from
[Magika](https://github.com/google/magika) (Apache-2.0).
