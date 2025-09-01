<!--
SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# air

A GUI client for OpenMLS

## Testing

```
# on first run
just frb-integrate
# on subsequent runs
just frb-generate
cd app
flutter test
```

The app uses snapshot testing, which captures screenshots and compares them with
golden files. These golden files are stored in Git LFS, so make sure to
configure it; otherwise, the files won't be available.

Due to differences in how operating systems render elements (e.g., aliasing,
font rendering, etc.), screenshots may vary across platforms. To ensure
consistency, snapshots are recorded on Linux. A CI job, Update Goldens, can be
triggered manually to update the golden files via CI.

Locally, you can run the following command to update the golden files:

```
cd app
fluter test --update-goldens
```

## Code organization

The app code is organized in the following way:

- Each feature is a separate module in the `lib` directory.
- Each module in the `lib` exports all its public APIs in the dart file with the
same name as the module. Other modules should import this dart file.
- Reusable widgets are in the `widgets` module.
- All styles and theme related code in the `theme` module.
- The module `core` contains the Rust generated code and extensions.

### Naming conventions

- `View` is a pure widget without any providers.
- `Container` wraps a `View` and creates all required providers.
- `ScreenView` is a root widget in the navigation without any providers.
- `Screen` wraps a `ScreenView` and creates all required providers. This is the
  actual class used in the navigation stack.
- `Pane` is a widget that is used in large screen layout and divide the screen into
  smaller sections.
