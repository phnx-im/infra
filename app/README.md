<!--
SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# prototype

A GUI client for OpenMLS

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
