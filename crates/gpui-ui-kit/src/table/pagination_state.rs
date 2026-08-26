/// Pagination state
#[derive(Debug, Clone, Default)]
pub struct PaginationState {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
}

impl PaginationState {
    /// Calculate total pages
    pub fn total_pages(&self) -> usize {
        if self.page_size == 0 {
            0
        } else {
            self.total_items.div_ceil(self.page_size)
        }
    }

    /// Calculate the displayed item range (1-based, inclusive).
    /// Returns `(0, 0)` when there are no items or page size is zero.
    pub fn page_range(&self) -> (usize, usize) {
        if self.page_size == 0 || self.total_items == 0 {
            (0, 0)
        } else {
            let page = self.current_page.min(self.total_pages().saturating_sub(1));
            let start = page * self.page_size + 1;
            let end = ((page + 1) * self.page_size).min(self.total_items);
            (start, end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::PaginationState;

    #[test]
    fn test_pagination_zero_page_size() {
        let p = PaginationState {
            current_page: 0,
            page_size: 0,
            total_items: 100,
        };
        let (start, end) = p.page_range();
        assert_eq!((start, end), (0, 0));
        assert_eq!(p.total_pages(), 0);
    }

    #[test]
    fn test_pagination_normal_page_range() {
        let p = PaginationState {
            current_page: 1,
            page_size: 10,
            total_items: 25,
        };
        let (start, end) = p.page_range();
        assert_eq!((start, end), (11, 20));
        assert_eq!(p.total_pages(), 3);
    }

    #[test]
    fn test_pagination_empty_and_out_of_range_ranges_are_not_misleading() {
        let empty = PaginationState {
            current_page: 0,
            page_size: 10,
            total_items: 0,
        };
        assert_eq!(empty.page_range(), (0, 0));

        let out_of_range = PaginationState {
            current_page: 99,
            page_size: 10,
            total_items: 25,
        };
        assert_eq!(out_of_range.page_range(), (21, 25));
    }
}
