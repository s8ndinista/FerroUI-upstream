// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

/// Stable identities for an items source, independent of its current order.
///
/// An [`IItemsList`](crate::IItemsList) exposes this optional contract through
/// `key_index_mapping`. Repeaters use it to retain the same control across a
/// reset or layout change. Keys must be unique within the source and must not
/// change while the corresponding item remains in the source.
pub trait IKeyIndexMapping {
    /// Returns the stable key of the item at `index`.
    fn key_from_index(&self, index: i32) -> String;

    /// Returns the current index for `key`, or `-1` when the item is absent.
    fn index_from_key(&self, key: &str) -> i32;
}
