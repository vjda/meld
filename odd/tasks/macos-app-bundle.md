# macOS application bundle (Meld_rs)

Package meld-rs as a native macOS `.app` bundle named `Meld_rs.app`, installable to
`/Applications`, exposing the `meld-rs` CLI on the user's PATH through a symlink into the
bundle. Local use only on arm64: no code signing, no notarization. Packaging lives in
standalone shell scripts under `scripts/`.

## Specs

- **S1.** User request verbatim: "Crear un bundle de una app de macos y luego yo pueda copiar
  a Applications. Su nombre será Meld_rs." — Produce `Meld_rs.app` (bundle directory name
  `Meld_rs`), installed to `/Applications`. Display name is "Meld RS" ("lo llamamos Meld_rs a
  nivel de bundle y nombre visible `Meld RS`").
- **S2.** User request verbatim: "El binario de meld-rs debería disponibilizarse a alguna ruta
  de mi PATH. Si no es viable, siempre puedo copiarlo yo a mano a ~/bin pero me gustaría que la
  app disponiblizara el binario también de algún modo (investíga cómo)" — Resolved per grill
  Q1: "Como indicas: symlink + script de instalación." The install step replaces the existing
  `~/bin/meld-rs` regular file (the user's manual copy) with a symlink into the installed app.
- **S3.** User request verbatim: "El icono de la app lo puedes extraer de data/meld_rs.icns." —
  The bundle icon comes from `data/meld_rs.icns` (CFBundleIconFile).
- **S4.** Packaging tooling (grill Q2, answer "a"): standalone shell scripts —
  `scripts/make-app.sh` assembles the bundle; `scripts/macos-install.sh` runs build + bundle +
  install + symlink in one step.
- **S5.** Scope limits (grill Q3 "solo local por ahora", Q4 "arm64 nativo de momento"): native
  arm64 binary only; no code signing, no notarization, no universal binary. Scripts must not
  require sudo (fail with clear guidance if `/Applications` is not writable).
- **S6.** Finder integration (grill Q5: "lo que indicas; integrar finder si ya lo acepta."
  and "sobre Q5 me quedo con i. No necesito arrastrar archivos, al menos de momento"): the
  bundle registers document types (files and folders) in `Info.plist` so Finder "Open With
  Meld_rs" works on cold launch; the CLI already accepts 1-3 paths (crates/meld/src/cli.rs).
  Opening documents while the app is already running (Apple Events) is explicitly out of scope.
- **S7.** PATH for external commands (grill Q6: "Quiero resolver el PATH con el bashrc que
  tengo en la home por lo que me quedo con la a para crear un script launcher que haga de
  pegamento."): `Contents/MacOS/Meld_rs` is a launcher script that sources `~/.bashrc` to
  resolve PATH (Finder-launched GUIs get launchd's minimal PATH, which does not include
  Homebrew tools such as `gh`, `glab`, `claude` invoked at runtime), appends fallback dirs
  (`/opt/homebrew/bin`, `/usr/local/bin`, `~/bin`, `~/.local/bin`), restores the original
  working directory and `exec`s the real `meld-rs` binary with the original arguments.
- **S8.** Identity: `CFBundleIdentifier` = `org.gnome.Meld.Rust` (matches the in-code
  `APP_ID`, crates/meld/src/app.rs). Version in `Info.plist` derives from the workspace
  `Cargo.toml` version (numeric `CFBundleShortVersionString`, pre-release suffix stripped).

## Tasks

| ID | Spec | Route | Commit |
|----|------|-------|--------|
| T1 | S1-S8 | RED tests in `scripts/test-macos-bundle.sh` first | (pending) |
| T2 | S1-S5, S7-S8 | `scripts/make-app.sh` (bundle + Info.plist + launcher) | (pending) |
| T3 | S2, S4-S5 | `scripts/macos-install.sh` (build + install + symlink) | (pending) |
| T4 | S1-S8 | README docs for macOS packaging | (pending) |
| T5 | S1-S8 | GREEN tests + end-to-end install run | (pending) |
| T6 | S1-S8 | Independent verification + native review (Risk: item 5, installer) | (pending) |

## Log

- **L1 (user request, verbatim):** "quiero que creamos una aplicación empaquetada para macos.
  Ahora mismo lo que hago es `cargo build --release` y me copio el binario
  `cp -p target/release/meld ~/bin/meld-rs` para tenerlo disponible en la terminal puesto que
  `~/bin` lo tengo añadido en mi PATH. Para lanzar la app desde el launcher de macos lo que
  tengo es una app que he creado con automator app de macos que ejecuta el binario por terminal
  pero es lo que me gustaría cambiar para que se sienta más nativa. Por tanto lo que quiero es:
  - Crear un bundle de una app de macos y luego yo pueda copiar a Applications. Su nombre será Meld_rs.
  - El binario de meld-rs debería disponibilizarse a alguna ruta de mi PATH. Si no es viable, siempre puedo copiarlo yo a mano a ~/bin pero me gustaría que la app disponiblizara el binario también de algún modo (investíga cómo)
  - El icono de la app lo puedes extraer de data/meld_rs.icns.
  Grill me si detectas ambiguedades, hay que tomar alguna decisión o centrar mejor el scope. Dame recomendaciones al respecto si detectas alguna mejora o cosas que sean interesantes añadir o tener en cuenta"
- **L2 (grill answers, verbatim excerpts):** Q1 "Como indicas: symlink + script de
  instalación." · Q2 "a" · Q3 "solo local por ahora" · Q4 "arm64 nativo de momento" · Q5 "lo
  que indicas; integrar finder si ya lo acepta." / "sobre Q5 me quedo con i. No necesito
  arrastrar archivos, al menos de momento" · Q6 "Quiero resolver el PATH con el bashrc que
  tengo en la home por lo que me quedo con la a para crear un script launcher que haga de
  pegamento." · nombres "lo llamamos Meld_rs a nivel de bundle y nombre visible `Meld RS`" ·
  bundle id "bundle id me gusta el que indicas" (= `org.gnome.Meld.Rust`).
- **L3 (exploration evidence):** CLI accepts 1-3 positional paths (crates/meld/src/cli.rs:121-122,
  limit enforced at cli.rs:182-187); `--newtab` IPC via Unix socket (crates/meld/src/instance.rs);
  all `data/` assets are embedded at compile time (meld-config/src/scheme.rs:24-47,
  crates/meld/src/app.rs:850-853, crates/meld/build.rs) — the bundle needs only binary + icns +
  Info.plist; xtask is gettext-only; no pre-existing packaging artifacts; APP_ID constant at
  crates/meld/src/app.rs:846.
- **L4 (bug found in e2e smoke, user-reported):** the launcher froze with no app when invoked as
  `~/bin/meld-rs`. Root cause: `HERE` derived from `$0` resolved to the symlink's own directory,
  so `exec "$HERE/meld-rs"` re-executed the launcher in an infinite loop (never reached the real
  binary). Fixed by resolving `$0` through symlink chains before computing `HERE`; regression test
  added (execute through an external symlink, bounded). Evidence: 30/30 checks, `~/bin/meld-rs
  --version` returns `meld 4.0.0-alpha.1`.
- **L5 (repo policy):** `data/meld_rs.icns` is NOT committed: the repo's pre-commit hook
  `check-added-large-files` rejects files over 500 KB (icon is 1146 KB). It remains an untracked
  local asset at `data/meld_rs.icns` that `scripts/make-app.sh` reads; use Git LFS or a hook
  change if it ever needs to be versioned.
