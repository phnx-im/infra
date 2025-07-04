// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::ensure;
use mimi_content::{
    MimiContent,
    content_container::{NestedPart, NestedPartContent, PartSemantics},
};

pub trait MimiContentExt {
    fn visit_attachments(
        &self,
        visitor: impl FnMut(&NestedPartContent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;

    fn visit_attachments_mut(
        &mut self,
        visitor: impl FnMut(&mut NestedPartContent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}

impl MimiContentExt for MimiContent {
    fn visit_attachments(
        &self,
        mut visitor: impl FnMut(&NestedPartContent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        visit_attachments_impl(&self.nested_part, &mut visitor, 0)
    }

    fn visit_attachments_mut(
        &mut self,
        mut visitor: impl FnMut(&mut NestedPartContent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        visit_attachments_mut_impl(&mut self.nested_part, &mut visitor, 0)
    }
}

const MAX_RECURSION_DEPTH: usize = 3;

fn visit_attachments_impl(
    part: &NestedPart,
    visitor: &mut impl FnMut(&NestedPartContent) -> anyhow::Result<()>,
    recursion_depth: usize,
) -> anyhow::Result<()> {
    ensure!(
        recursion_depth < MAX_RECURSION_DEPTH,
        "Failed to handle attachment due to maximum recursion depth reached"
    );

    match &part.part {
        external_part @ NestedPartContent::ExternalPart { .. } => {
            visitor(external_part)?;
        }
        NestedPartContent::MultiPart {
            part_semantics: PartSemantics::ProcessAll,
            parts,
        } => {
            for part in parts {
                visit_attachments_impl(part, visitor, recursion_depth + 1)?;
            }
        }
        _ => (),
    }

    Ok(())
}

fn visit_attachments_mut_impl(
    part: &mut NestedPart,
    visitor: &mut impl FnMut(&mut NestedPartContent) -> anyhow::Result<()>,
    recursion_depth: usize,
) -> anyhow::Result<()> {
    ensure!(
        recursion_depth < MAX_RECURSION_DEPTH,
        "Failed to handle attachment due to maximum recursion depth reached"
    );

    match &mut part.part {
        external_part @ NestedPartContent::ExternalPart { .. } => {
            visitor(external_part)?;
        }
        NestedPartContent::MultiPart {
            part_semantics: PartSemantics::ProcessAll,
            parts,
        } => {
            for part in parts {
                visit_attachments_mut_impl(part, visitor, recursion_depth + 1)?;
            }
        }
        _ => (),
    }

    Ok(())
}
