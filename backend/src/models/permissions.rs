use std::{fmt, ops::BitOr};

use serde::{Deserialize, Serialize};

/// A usergroup's permission bit flags. A user's effective permissions are the bitwise OR
/// of all their usergroups' permissions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Permissions(i64);

impl Permissions {
    /// If clear, acts as a global ban: only support pages can be viewed and used.
    pub const SITE_ACCESS: Self = Self(0x1);
    pub const USER_POSTS: Self = Self(0x2);
    pub const THREAD_MANAGEMENT: Self = Self(0x4);
    pub const CATEGORY_MANAGEMENT: Self = Self(0x8);
    pub const SECTION_MANAGEMENT: Self = Self(0x10);
    pub const USER_MANAGEMENT: Self = Self(0x20);
    pub const ADMIN_ACCESS: Self = Self(0x40);
    /// Dangerous, site-wide actions such as clearing all data or triggering a panic.
    pub const ELEVATED_CONTROL: Self = Self(0x80);
    pub const USERGROUP_MANAGEMENT: Self = Self(0x100);
    pub const USER_MESSAGES: Self = Self(0x200);
    pub const SERVER_MANAGEMENT: Self = Self(0x400);
    pub const COMMUNITY_MANAGEMENT: Self = Self(0x800);
    /// Can see and respond to every support ticket, and manage FAQs.
    pub const SUPPORT_TEAM: Self = Self(0x1000);
    /// Manages item stores, their items and discounts (not a store's grantable
    /// usergroups, which need Elevated Control).
    pub const STORE_MANAGEMENT: Self = Self(0x2000);
    /// Sees receipts and revenue, and issues refunds.
    pub const STORE_FINANCE: Self = Self(0x4000);

    /// Every permission bit, with its display name.
    pub const FLAGS: [(Self, &'static str); 15] = [
        (Self::SITE_ACCESS, "Site Access"),
        (Self::USER_POSTS, "User Posts"),
        (Self::THREAD_MANAGEMENT, "Thread Management"),
        (Self::CATEGORY_MANAGEMENT, "Category Management"),
        (Self::SECTION_MANAGEMENT, "Section Management"),
        (Self::USER_MANAGEMENT, "User Management"),
        (Self::ADMIN_ACCESS, "Admin Access"),
        (Self::ELEVATED_CONTROL, "Elevated Control"),
        (Self::USERGROUP_MANAGEMENT, "Usergroup Management"),
        (Self::USER_MESSAGES, "User Messages"),
        (Self::SERVER_MANAGEMENT, "Server Management"),
        (Self::COMMUNITY_MANAGEMENT, "Community Management"),
        (Self::SUPPORT_TEAM, "Support Team"),
        (Self::STORE_MANAGEMENT, "Store Management"),
        (Self::STORE_FINANCE, "Store Finance"),
    ];

    pub const ALL: Self = {
        let mut bits = 0;
        let mut i = 0;
        while i < Self::FLAGS.len() {
            bits |= Self::FLAGS[i].0.0;
            i += 1;
        }
        Self(bits)
    };

    /// What every user gets: site access, posting and private messages.
    pub const BASIC: Self = Self::SITE_ACCESS
        .union(Self::USER_POSTS)
        .union(Self::USER_MESSAGES);

    pub const fn from_bits(bits: i64) -> Self {
        Self(bits)
    }

    /// Bits set in exactly one of `self` and `other`.
    pub const fn symmetric_difference(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    /// Bits set in `self` but not in `other`.
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Display name of a single flag, e.g. `"Admin Access"`.
    pub fn name(self) -> &'static str {
        Self::FLAGS
            .into_iter()
            .find(|(flag, _)| *flag == self)
            .map_or("Unknown", |(_, name)| name)
    }

    /// Names of the set permissions, in bit order.
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        Self::FLAGS
            .into_iter()
            .filter(move |(flag, _)| self.contains(*flag))
            .map(|(_, name)| name)
    }
}

impl BitOr for Permissions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// `0x203 (Site Access, User Posts, User Messages)`
impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)?;
        let names: Vec<_> = self.names().collect();
        if !names.is_empty() {
            write!(f, " ({})", names.join(", "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_covers_every_documented_bit() {
        assert_eq!(Permissions::ALL.0, 0x7FFF);
    }

    #[test]
    fn basic() {
        assert_eq!(Permissions::BASIC.0, 0x203);
        assert!(Permissions::BASIC.contains(Permissions::SITE_ACCESS));
        assert!(!Permissions::BASIC.contains(Permissions::ADMIN_ACCESS));
    }

    #[test]
    fn display() {
        assert_eq!(Permissions::default().to_string(), "0x0");
        assert_eq!(
            Permissions::BASIC.to_string(),
            "0x203 (Site Access, User Posts, User Messages)"
        );
    }
}
