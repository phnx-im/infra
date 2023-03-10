# SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

reuse download AGPL-3.0-or-later
reuse annotate -c "Phoenix R&D GmbH <hello@phnx.im>" -l AGPL-3.0-or-later --skip-existing -r * .gitignore .github
reuse lint
