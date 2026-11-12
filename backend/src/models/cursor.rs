//! Keyset pagination over `(created_at, id)`, newest first, shared by the lists that page
//! with `limit` and `cursor` (notifications, private messages). Rows arriving while a
//! client pages can't shift or repeat a page, unlike `OFFSET`.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Where a page ended: the `created_at` and `id` of its last row. Sent to clients as an
/// opaque string; the next page holds the rows before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub created_at: DateTime<Utc>,
    pub id: i64,
}

impl Cursor {
    pub fn parse(value: &str) -> Option<Self> {
        let (micros, id) = value.split_once('-')?;
        Some(Self {
            created_at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: id.parse().ok()?,
        })
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Postgres timestamps have microsecond precision, so this round-trips exactly.
        write!(f, "{}-{}", self.created_at.timestamp_micros(), self.id)
    }
}

/// One page of a list: `{ items, next_cursor }`.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Pass as `cursor` for the next page; `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Cuts `rows`, fetched with `LIMIT limit + 1`, down to `limit`, and returns the cursor
/// of the next page: the last row kept, if the extra row showed there is more.
pub fn paginate<T>(rows: &mut Vec<T>, limit: i64, key: impl Fn(&T) -> Cursor) -> Option<String> {
    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit.max(0) as usize);
    rows.last()
        .filter(|_| has_more)
        .map(|last| key(last).to_string())
}

impl<T> Page<T> {
    /// A page from `rows` fetched with `LIMIT limit + 1`; see [`paginate`].
    pub fn new(mut rows: Vec<T>, limit: i64, key: impl Fn(&T) -> Cursor) -> Self {
        let next_cursor = paginate(&mut rows, limit, key);
        Self {
            items: rows,
            next_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(id: i64) -> Cursor {
        Cursor {
            created_at: DateTime::from_timestamp_micros(1_794_000_000_000_000 + id).unwrap(),
            id,
        }
    }

    #[test]
    fn cursors() {
        let cursor = Cursor {
            created_at: DateTime::from_timestamp_micros(1_794_000_000_123_456).unwrap(),
            id: 42,
        };
        assert_eq!(cursor.to_string(), "1794000000123456-42");
        assert_eq!(Cursor::parse(&cursor.to_string()), Some(cursor));
        assert_eq!(Cursor::parse(""), None);
        assert_eq!(Cursor::parse("42"), None);
        assert_eq!(Cursor::parse("abc-1"), None);
        assert_eq!(Cursor::parse("1794000000123456-"), None);
        assert_eq!(Cursor::parse("1794000000123456-x"), None);
    }

    #[test]
    fn pages() {
        let page = Page::new(vec![5, 4, 3], 2, |&id| at(id));
        assert_eq!(page.items, vec![5, 4]);
        assert_eq!(page.next_cursor, Some(at(4).to_string()));

        let page = Page::new(vec![5, 4], 2, |&id| at(id));
        assert_eq!(page.items, vec![5, 4]);
        assert_eq!(page.next_cursor, None, "no extra row: last page");

        let page = Page::new(Vec::<i64>::new(), 25, |&id| at(id));
        assert!(page.items.is_empty());
        assert_eq!(page.next_cursor, None);
    }
}
