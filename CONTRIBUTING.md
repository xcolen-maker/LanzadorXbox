# Contributing to Lanzador Xbox

Thanks for wanting to help! This is a small project, so anything from a typo fix to a new
translation is welcome.

## Ways to help (no code needed)

- **Report a bug** or a confusing screen: open an [issue](../../issues). Say what you did,
  what you expected and what happened. A screenshot helps.
- **Test an emulator fork** and tell us if it builds and runs from the launcher.
- **Improve a translation.** The interface is in English, Spanish, Portuguese, French and
  Russian, and native speakers can spot what a machine can't.
- **Improve the README** or the screenshots.

## What is and isn't in scope

In scope: installing, updating and running emulators of the supported kinds (**VIctory**
and **xcl** forks), detecting existing copies, the interface and its translations.

Out of scope: distributing games, game files, keys or anything copyrighted, and support for
emulators that are not based on VIctory or xcl. Pull requests that add those will be
closed.

## Building and testing

You need Rust (stable) and, on Windows, the Visual Studio Build Tools.

```
cargo build --release
cargo test
```

`cargo test` also checks that every translation is complete. See the README for details.

## Adding or fixing a translation

All texts live in `src/i18n.rs`. The Spanish text is the key, and each entry lists its
translations in this order: English, Portuguese, French, Russian. Keep the same number of
`{}` placeholders in every language; `cargo test` fails if they differ.

To add a text to the interface, wrap it in `tr("...")` (or `trf!("...", args)` when it has
`{}` placeholders) and add its entry to the table.

## Sending a pull request

1. Fork the repository and create a branch from `main`.
2. Keep the change small and focused: one thing per pull request.
3. Run `cargo build --release` and `cargo test` before you push.
4. Describe what changed and why. If it changes how the launcher looks, include a
   screenshot.

## Privacy

Please do not include personal data in commits, issues or screenshots: your real name,
email, Windows user name or personal folder paths. Use a noreply email address for commits
(GitHub offers one in **Settings → Emails**), and crop or blur screenshots that show your
user name.

## Licence

By contributing you agree that your work is released under the project's [MIT licence](LICENSE).

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).
